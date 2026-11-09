use super::{PidfdOperations, error_is_process_gone};
use anyhow::Result;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const DESCRIPTOR_RANGE_CONTEXT: &str = "Linux pidfd exceeds descriptor range";
const PID_RANGE_CONTEXT: &str = "Linux process id exceeds platform range";
const PIDFD_OPEN_FLAGS: libc::c_uint = 0;
const PIDFD_SIGNAL_FLAGS: libc::c_uint = 0;
const POLL_DESCRIPTOR_COUNT: libc::nfds_t = 1;
const POLL_ERROR_EVENTS: libc::c_short = libc::POLLERR | libc::POLLNVAL;
const POLL_EXIT_EVENTS: libc::c_short = libc::POLLIN | libc::POLLHUP;
const POLL_FAILURE: libc::c_int = -1;
const POLL_NO_EVENTS: libc::c_short = 0;
const POLL_READY_RESULT: libc::c_int = 1;
const POLL_TIMEOUT_IMMEDIATE: libc::c_int = 0;
const POLL_UNEXPECTED_DETAIL: &str = "pidfd poll returned unexpected readiness";
const SYSCALL_FAILURE: libc::c_long = -1;

pub(super) struct KernelPidfdOperations;

impl PidfdOperations for KernelPidfdOperations {
    type Descriptor = OwnedFd;

    fn open(&self, pid: u32) -> io::Result<Self::Descriptor> {
        let pid = libc::pid_t::try_from(pid)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, PID_RANGE_CONTEXT))?;
        let result = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, PIDFD_OPEN_FLAGS) };
        if result == SYSCALL_FAILURE {
            return Err(io::Error::last_os_error());
        }
        let descriptor = libc::c_int::try_from(result)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, DESCRIPTOR_RANGE_CONTEXT))?;
        Ok(unsafe { OwnedFd::from_raw_fd(descriptor) })
    }

    fn start_time(&self, pid: u32) -> Result<Option<u64>> {
        match super::super::super::process_start_time(pid) {
            Ok(start_time) => Ok(Some(start_time)),
            Err(error) if error_is_process_gone(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn exited(&self, descriptor: &Self::Descriptor) -> io::Result<bool> {
        poll_exited(descriptor)
    }

    fn send(&self, descriptor: &Self::Descriptor, signal: i32) -> io::Result<()> {
        let result = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                descriptor.as_raw_fd(),
                signal,
                std::ptr::null::<libc::siginfo_t>(),
                PIDFD_SIGNAL_FLAGS,
            )
        };
        if result == SYSCALL_FAILURE {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

fn poll_exited(descriptor: &OwnedFd) -> io::Result<bool> {
    let mut candidate = libc::pollfd {
        fd: descriptor.as_raw_fd(),
        events: libc::POLLIN,
        revents: POLL_NO_EVENTS,
    };
    loop {
        let result = unsafe {
            libc::poll(
                &mut candidate,
                POLL_DESCRIPTOR_COUNT,
                POLL_TIMEOUT_IMMEDIATE,
            )
        };
        if result == POLL_FAILURE && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted
        {
            continue;
        }
        if result == POLL_FAILURE {
            return Err(io::Error::last_os_error());
        }
        if result == libc::c_int::default() {
            return Ok(false);
        }
        let exit_ready = candidate.revents & POLL_EXIT_EVENTS != POLL_NO_EVENTS;
        if result != POLL_READY_RESULT
            || candidate.revents & POLL_ERROR_EVENTS != POLL_NO_EVENTS
            || !exit_ready
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                POLL_UNEXPECTED_DETAIL,
            ));
        }
        return Ok(true);
    }
}
