use super::constants::*;
use std::io;
use std::os::fd::AsRawFd;
use std::time::Instant;

pub(super) fn readable(
    stream: &std::os::unix::net::UnixStream,
    deadline: Option<Instant>,
) -> io::Result<()> {
    if nonblocking(stream)? {
        return Err(io::ErrorKind::WouldBlock.into());
    }
    loop {
        let milliseconds = poll_milliseconds(deadline)?;
        let mut descriptor = libc::pollfd {
            fd: stream.as_raw_fd(),
            events: libc::POLLIN | libc::POLLHUP,
            revents: libc::c_short::default(),
        };
        let result = unsafe { libc::poll(&mut descriptor, POLL_DESCRIPTOR_COUNT, milliseconds) };
        if result == POLL_READY {
            return Ok(());
        }
        if result == POLL_TIMEOUT {
            return Err(io::Error::new(io::ErrorKind::TimedOut, WAIT_TIMEOUT_DETAIL));
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

fn nonblocking(stream: &std::os::unix::net::UnixStream) -> io::Result<bool> {
    let flags = unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_GETFL) };
    if flags == FCNTL_FAILURE {
        return Err(io::Error::last_os_error());
    }
    Ok(flags & libc::O_NONBLOCK != libc::c_int::default())
}

fn poll_milliseconds(deadline: Option<Instant>) -> io::Result<libc::c_int> {
    let Some(deadline) = deadline else {
        return Ok(INFINITE_POLL_MILLISECONDS);
    };
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, WAIT_TIMEOUT_DETAIL))?;
    let floor = remaining.as_millis();
    let rounded = if std::time::Duration::from_millis(floor as u64) < remaining {
        floor.saturating_add(MINIMUM_POLL_MILLISECONDS)
    } else {
        floor
    };
    libc::c_int::try_from(rounded.max(MINIMUM_POLL_MILLISECONDS)).map_err(io::Error::other)
}
