use super::pty_output_progress::{OutputDescriptor, OutputQueueState};
use anyhow::{Context, Result};
use portable_pty::MasterPty;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};

pub(crate) const MINIMUM_DUPLICATED_FD: libc::c_int = 0;
pub(crate) const SYSCALL_FAILURE: libc::c_int = -1;
pub(crate) const MASTER_FD_UNAVAILABLE_DETAIL: &str =
    "PTY output-progress descriptor is unavailable";
pub(crate) const MASTER_DUPLICATE_CONTEXT: &str = "duplicate PTY master for output progress";
pub(crate) const MASTER_RESUME_CONTEXT: &str = "resume PTY output during process settlement";
pub(crate) const MASTER_QUEUE_CONTEXT: &str = "observe PTY output queue during process settlement";
pub(crate) const MASTER_QUEUE_INVALID_DETAIL: &str = "PTY output queue reported a negative length";
pub(crate) const MASTER_EOF_POLL_CONTEXT: &str = "validate current PTY kernel EOF";
pub(crate) const MASTER_EOF_POLL_INVALID_DETAIL: &str = "PTY EOF descriptor is invalid";
pub(crate) const MASTER_EOF_BYTES_CONTEXT: &str = "observe readable PTY bytes while validating EOF";
pub(crate) const MASTER_EOF_BYTES_INVALID_DETAIL: &str =
    "PTY EOF descriptor reported negative readable bytes";
pub(crate) const POLL_DESCRIPTOR_COUNT: libc::nfds_t = 1;
pub(crate) const POLL_READY_DESCRIPTOR_COUNT: libc::c_int = 1;
pub(crate) const POLL_TIMEOUT: libc::c_int = 0;
pub(crate) const POLL_TIMEOUT_RESULT: libc::c_int = 0;
pub(crate) const EMPTY_POLL_EVENTS: libc::c_short = 0;
#[cfg(target_os = "macos")]
pub(crate) const MASTER_START_CONTEXT: &str = "start PTY output during process settlement";

pub(crate) fn capture(master: &dyn MasterPty) -> Result<OutputDescriptor> {
    let fd = master.as_raw_fd().context(MASTER_FD_UNAVAILABLE_DETAIL)?;
    let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, MINIMUM_DUPLICATED_FD) };
    if duplicate == SYSCALL_FAILURE {
        return Err(io::Error::last_os_error()).context(MASTER_DUPLICATE_CONTEXT);
    }
    let file = unsafe { File::from_raw_fd(duplicate) };
    super::nonblocking_pty::configure_nonblocking_fd(file.as_raw_fd())?;
    Ok(OutputDescriptor::Native(file))
}

pub(crate) fn resume(file: &File) -> Result<()> {
    let fd = file.as_raw_fd();
    let flow = terminal_operation(
        unsafe { libc::tcflow(fd, libc::TCOON) },
        MASTER_RESUME_CONTEXT,
    );
    #[cfg(target_os = "macos")]
    {
        let start = terminal_operation(
            unsafe { libc::ioctl(fd, libc::TIOCSTART.into()) },
            MASTER_START_CONTEXT,
        );
        require_any_resume_operation(flow, start)
    }
    #[cfg(not(target_os = "macos"))]
    {
        flow
    }
}

pub(crate) fn validates_kernel_pty_eof(file: &File) -> Result<bool> {
    #[cfg(target_os = "macos")]
    if output_queue_state(file)? == OutputQueueState::Pending {
        return Ok(false);
    }
    let events = poll_events(file)?;
    if events & libc::POLLIN != EMPTY_POLL_EVENTS && readable_bytes(file)? != libc::c_int::default()
    {
        return Ok(false);
    }
    Ok(events & libc::POLLHUP != EMPTY_POLL_EVENTS)
}

pub(crate) fn poll_events(file: &File) -> Result<libc::c_short> {
    let mut descriptor = libc::pollfd {
        fd: file.as_raw_fd(),
        events: libc::POLLIN | libc::POLLHUP | libc::POLLERR,
        revents: EMPTY_POLL_EVENTS,
    };
    loop {
        descriptor.revents = EMPTY_POLL_EVENTS;
        let result = unsafe { libc::poll(&mut descriptor, POLL_DESCRIPTOR_COUNT, POLL_TIMEOUT) };
        if result == SYSCALL_FAILURE {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error).context(MASTER_EOF_POLL_CONTEXT);
        }
        if result == POLL_TIMEOUT_RESULT {
            return Ok(EMPTY_POLL_EVENTS);
        }
        anyhow::ensure!(
            result == POLL_READY_DESCRIPTOR_COUNT,
            MASTER_EOF_POLL_CONTEXT
        );
        anyhow::ensure!(
            descriptor.revents & libc::POLLNVAL == EMPTY_POLL_EVENTS,
            MASTER_EOF_POLL_INVALID_DETAIL
        );
        return Ok(descriptor.revents);
    }
}

pub(crate) fn validates_consumed_kernel_pty_eof(file: &File) -> Result<bool> {
    validates_kernel_pty_eof(file)
}

pub(crate) fn readable_bytes(file: &File) -> Result<libc::c_int> {
    let mut readable = libc::c_int::default();
    let result = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            libc::FIONREAD.into(),
            std::ptr::from_mut(&mut readable),
        )
    };
    if result == SYSCALL_FAILURE {
        return Err(io::Error::last_os_error()).context(MASTER_EOF_BYTES_CONTEXT);
    }
    anyhow::ensure!(
        readable >= libc::c_int::default(),
        MASTER_EOF_BYTES_INVALID_DETAIL
    );
    Ok(readable)
}

#[cfg(target_os = "macos")]
pub(crate) fn require_any_resume_operation(flow: Result<()>, start: Result<()>) -> Result<()> {
    flow.or(start)
}

#[cfg(target_os = "macos")]
pub(crate) fn output_queue_state(file: &File) -> Result<OutputQueueState> {
    let mut queued = libc::c_int::default();
    let result = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            libc::TIOCOUTQ.into(),
            std::ptr::from_mut(&mut queued),
        )
    };
    if result == SYSCALL_FAILURE {
        return Err(io::Error::last_os_error()).context(MASTER_QUEUE_CONTEXT);
    }
    anyhow::ensure!(
        queued >= libc::c_int::default(),
        MASTER_QUEUE_INVALID_DETAIL
    );
    Ok(if queued == libc::c_int::default() {
        OutputQueueState::Empty
    } else {
        OutputQueueState::Pending
    })
}

#[cfg(target_os = "linux")]
#[path = "pty_output_terminal_linux.rs"]
mod linux;
#[cfg(target_os = "linux")]
pub(crate) use linux::output_queue_state;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(crate) fn output_queue_state(_file: &File) -> Result<OutputQueueState> {
    Err(io::Error::new(io::ErrorKind::Unsupported, MASTER_QUEUE_CONTEXT).into())
}

pub(crate) fn terminal_operation(result: libc::c_int, context: &'static str) -> Result<()> {
    if result != SYSCALL_FAILURE {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::EIO) {
        return Ok(());
    }
    Err(error).context(context)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) fn is_kernel_pty_eof_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::EIO)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(crate) fn is_kernel_pty_eof_error(_error: &io::Error) -> bool {
    false
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/output/pty_output_terminal_tests.rs"]
mod tests;
