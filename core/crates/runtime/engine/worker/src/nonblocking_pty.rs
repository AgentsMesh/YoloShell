use super::{
    PTY_FD_FLAGS_READ_CONTEXT, PTY_FD_NONBLOCK_CONTEXT, PTY_FD_UNAVAILABLE_DETAIL,
    PTY_READER_POLL_CONTEXT, PTY_READER_POLL_INVALID_DETAIL, PTY_READER_POLL_UNEXPECTED_DETAIL,
};
use portable_pty::MasterPty;
use std::io;
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};

const SYSCALL_FAILURE: i32 = -1;
const POLL_READY_DESCRIPTOR_COUNT: i32 = 1;
const POLL_DESCRIPTOR_COUNT: libc::nfds_t = 1;
const PTY_READER_POLL_TIMEOUT_MS: i32 = 50;
const POLL_TIMEOUT_RESULT: i32 = 0;
const EMPTY_POLL_EVENTS: i16 = 0;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum PtyReadiness {
    Readable,
    Closed,
    Cancelled,
    Probe,
}

pub(super) fn configure_nonblocking_pty(master: &dyn MasterPty) -> io::Result<RawFd> {
    let fd = master
        .as_raw_fd()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, PTY_FD_UNAVAILABLE_DETAIL))?;
    configure_nonblocking_fd(fd)?;
    Ok(fd)
}

pub(super) fn configure_nonblocking_fd(fd: RawFd) -> io::Result<()> {
    let flags = fcntl_get_flags(fd)?;
    if flags & libc::O_NONBLOCK == 0 {
        fcntl_set_flags(fd, flags | libc::O_NONBLOCK, PTY_FD_NONBLOCK_CONTEXT)?;
    }
    Ok(())
}

pub(super) fn wait_for_pty_readiness(
    fd: RawFd,
    cancelled: &AtomicBool,
    wakeup_requested: &AtomicBool,
) -> io::Result<PtyReadiness> {
    wait_for_pty_readiness_with(fd, cancelled, wakeup_requested, poll_descriptor)
}

fn wait_for_pty_readiness_with<Poll>(
    fd: RawFd,
    cancelled: &AtomicBool,
    wakeup_requested: &AtomicBool,
    mut poll: Poll,
) -> io::Result<PtyReadiness>
where
    Poll: FnMut(&mut libc::pollfd, &AtomicBool) -> i32,
{
    let mut descriptor = libc::pollfd {
        fd,
        events: libc::POLLIN | libc::POLLHUP | libc::POLLERR,
        revents: EMPTY_POLL_EVENTS,
    };
    loop {
        descriptor.revents = EMPTY_POLL_EVENTS;
        if cancelled.load(Ordering::Acquire) {
            return Ok(PtyReadiness::Cancelled);
        }
        if wakeup_requested.swap(false, Ordering::AcqRel) {
            return Ok(PtyReadiness::Probe);
        }
        let result = poll(&mut descriptor, cancelled);
        if result == POLL_READY_DESCRIPTOR_COUNT {
            return classify_poll_events(descriptor.revents);
        }
        if result == SYSCALL_FAILURE {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(with_context(PTY_READER_POLL_CONTEXT, error));
        }
        if result == POLL_TIMEOUT_RESULT {
            continue;
        }
        return Err(io::Error::other(PTY_READER_POLL_UNEXPECTED_DETAIL));
    }
}

fn poll_descriptor(descriptor: &mut libc::pollfd, _cancelled: &AtomicBool) -> i32 {
    unsafe {
        libc::poll(
            descriptor,
            POLL_DESCRIPTOR_COUNT,
            PTY_READER_POLL_TIMEOUT_MS,
        )
    }
}

fn classify_poll_events(events: i16) -> io::Result<PtyReadiness> {
    if events & libc::POLLNVAL != 0 {
        return Err(io::Error::other(PTY_READER_POLL_INVALID_DETAIL));
    }
    if events & libc::POLLIN != 0 {
        return Ok(PtyReadiness::Readable);
    }
    if events & (libc::POLLHUP | libc::POLLERR) != 0 {
        return Ok(PtyReadiness::Closed);
    }
    Err(io::Error::other(PTY_READER_POLL_UNEXPECTED_DETAIL))
}

fn fcntl_get_flags(fd: RawFd) -> io::Result<i32> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == SYSCALL_FAILURE {
        return Err(with_context(
            PTY_FD_FLAGS_READ_CONTEXT,
            io::Error::last_os_error(),
        ));
    }
    Ok(flags)
}

fn fcntl_set_flags(fd: RawFd, flags: i32, context: &str) -> io::Result<()> {
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags) } == SYSCALL_FAILURE {
        return Err(with_context(context, io::Error::last_os_error()));
    }
    Ok(())
}

fn with_context(context: &str, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{context}: {error}"))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/nonblocking/nonblocking_pty_tests.rs"]
mod tests;
