use super::*;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::path::PathBuf;

#[cfg(target_os = "linux")]
#[path = "pty_output_terminal_linux_tests.rs"]
mod linux_tests;

const INVALID_MASTER_OPERATION: &str = "invalid master operation";
const INVALID_MASTER_CLOSE_SUCCESS: libc::c_int = 0;

struct InvalidMaster(Option<RawFd>);

impl MasterPty for InvalidMaster {
    fn resize(&self, _size: portable_pty::PtySize) -> Result<()> {
        anyhow::bail!(INVALID_MASTER_OPERATION)
    }

    fn get_size(&self) -> Result<portable_pty::PtySize> {
        anyhow::bail!(INVALID_MASTER_OPERATION)
    }

    fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>> {
        anyhow::bail!(INVALID_MASTER_OPERATION)
    }

    fn take_writer(&self) -> Result<Box<dyn Write + Send>> {
        anyhow::bail!(INVALID_MASTER_OPERATION)
    }

    fn process_group_leader(&self) -> Option<libc::pid_t> {
        None
    }

    fn as_raw_fd(&self) -> Option<RawFd> {
        self.0
    }

    fn tty_name(&self) -> Option<PathBuf> {
        None
    }
}

#[test]
fn capture_rejects_missing_and_closed_master_descriptors() {
    let missing = capture(&InvalidMaster(None)).err().unwrap();
    assert!(missing.to_string().contains(MASTER_FD_UNAVAILABLE_DETAIL));

    let (stream, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let duplicate = unsafe { libc::dup(stream.as_raw_fd()) };
    assert_ne!(duplicate, SYSCALL_FAILURE);
    assert_eq!(
        unsafe { libc::close(duplicate) },
        INVALID_MASTER_CLOSE_SUCCESS
    );
    let closed = capture(&InvalidMaster(Some(duplicate))).err().unwrap();
    assert!(closed.to_string().contains(MASTER_DUPLICATE_CONTEXT));
}

#[cfg(target_os = "macos")]
const PORTABLE_RESUME_FAILURE: &str = "portable resume failed";

#[cfg(target_os = "macos")]
#[test]
fn native_resume_success_supersedes_portable_failure() {
    let flow = Err(anyhow::anyhow!(PORTABLE_RESUME_FAILURE));
    assert!(require_any_resume_operation(flow, Ok(())).is_ok());
}

#[cfg(target_os = "macos")]
#[test]
fn non_terminal_queue_observation_fails_closed() {
    let file = tempfile::tempfile().unwrap();
    assert!(output_queue_state(&file).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn terminal_operation_accepts_native_eof_but_preserves_other_errors() {
    set_terminal_errno(libc::EIO);
    assert!(terminal_operation(SYSCALL_FAILURE, MASTER_RESUME_CONTEXT).is_ok());

    set_terminal_errno(libc::EBADF);
    let error = terminal_operation(SYSCALL_FAILURE, MASTER_RESUME_CONTEXT).unwrap_err();
    assert!(error.to_string().contains(MASTER_RESUME_CONTEXT));
}

#[cfg(target_os = "macos")]
fn set_terminal_errno(value: libc::c_int) {
    unsafe { *libc::__error() = value };
}
