use super::*;
use std::io;
use std::mem::size_of;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;

const INVALID_PEER_PID: libc::pid_t = 0;
const SOCKET_OPTION_SUCCESS: libc::c_int = 0;
const PEER_PID_RANGE_DETAIL: &str = "cleanup peer PID is out of range";
const PEER_PID_INVALID_DETAIL: &str = "cleanup peer PID is invalid";
const PEER_PID_UNSUPPORTED_DETAIL: &str = "cleanup peer PID credentials are unsupported";

#[cfg(target_os = "macos")]
const PEER_PID_LENGTH: usize = size_of::<libc::pid_t>();
#[cfg(target_os = "linux")]
const PEER_CREDENTIAL_LENGTH: usize = size_of::<libc::ucred>();

pub(super) fn matches_runtime(stream: &UnixStream, record: &RuntimeRecord) -> bool {
    let Ok(pid) = peer_pid(stream) else {
        return false;
    };
    let Ok(identity) = yolo_process_test_support::ProcessIdentity::capture(pid) else {
        return false;
    };
    identity.pid == record.worker_pid && identity.start_time == record.worker_start_time
}

fn peer_pid(stream: &UnixStream) -> io::Result<u32> {
    let descriptor = stream.as_raw_fd();
    #[cfg(target_os = "macos")]
    {
        let mut pid = INVALID_PEER_PID;
        let mut length = socket_length(PEER_PID_LENGTH)?;
        let result = unsafe {
            libc::getsockopt(
                descriptor,
                libc::SOL_LOCAL,
                libc::LOCAL_PEERPID,
                (&raw mut pid).cast(),
                &mut length,
            )
        };
        ensure_option_result(result, length, PEER_PID_LENGTH)?;
        return checked_pid(pid);
    }
    #[cfg(target_os = "linux")]
    {
        let mut credentials = libc::ucred {
            pid: INVALID_PEER_PID,
            uid: 0,
            gid: 0,
        };
        let mut length = socket_length(PEER_CREDENTIAL_LENGTH)?;
        let result = unsafe {
            libc::getsockopt(
                descriptor,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&raw mut credentials).cast(),
                &mut length,
            )
        };
        ensure_option_result(result, length, PEER_CREDENTIAL_LENGTH)?;
        return checked_pid(credentials.pid);
    }
    #[allow(unreachable_code)]
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        PEER_PID_UNSUPPORTED_DETAIL,
    ))
}

fn socket_length(length: usize) -> io::Result<libc::socklen_t> {
    libc::socklen_t::try_from(length)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, PEER_PID_RANGE_DETAIL))
}

fn ensure_option_result(
    result: libc::c_int,
    actual_length: libc::socklen_t,
    expected_length: usize,
) -> io::Result<()> {
    if result != SOCKET_OPTION_SUCCESS {
        return Err(io::Error::last_os_error());
    }
    if usize::try_from(actual_length).ok() != Some(expected_length) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            PEER_PID_INVALID_DETAIL,
        ));
    }
    Ok(())
}

fn checked_pid(pid: libc::pid_t) -> io::Result<u32> {
    if pid <= INVALID_PEER_PID {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            PEER_PID_INVALID_DETAIL,
        ));
    }
    u32::try_from(pid)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, PEER_PID_RANGE_DETAIL))
}
