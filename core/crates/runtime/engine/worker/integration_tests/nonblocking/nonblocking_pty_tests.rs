use super::*;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;

const UNEXPECTED_POLL_RESULT: i32 = POLL_READY_DESCRIPTOR_COUNT + 1;

fn active_wait() -> AtomicBool {
    AtomicBool::new(false)
}

fn inactive_probe() -> AtomicBool {
    AtomicBool::new(false)
}

#[test]
fn configuration_enables_nonblocking_status() {
    let (stream, _) = UnixStream::pair().unwrap();
    configure_nonblocking_fd(stream.as_raw_fd()).unwrap();
    assert_ne!(
        fcntl_get_flags(stream.as_raw_fd()).unwrap() & libc::O_NONBLOCK,
        0
    );
}

#[test]
fn configuration_preserves_an_existing_nonblocking_status() {
    let (stream, _) = UnixStream::pair().unwrap();
    let original = fcntl_get_flags(stream.as_raw_fd()).unwrap();
    fcntl_set_flags(
        stream.as_raw_fd(),
        original | libc::O_NONBLOCK,
        PTY_FD_NONBLOCK_CONTEXT,
    )
    .unwrap();
    configure_nonblocking_fd(stream.as_raw_fd()).unwrap();
    assert_ne!(
        fcntl_get_flags(stream.as_raw_fd()).unwrap() & libc::O_NONBLOCK,
        0
    );
}

#[test]
fn readiness_wait_reports_data() {
    let (mut writer, reader) = UnixStream::pair().unwrap();
    writer.write_all(b"ready").unwrap();
    assert_eq!(
        wait_for_pty_readiness(reader.as_raw_fd(), &active_wait(), &inactive_probe()).unwrap(),
        PtyReadiness::Readable
    );
}

#[test]
fn readiness_wait_honors_cancellation_without_descriptor_activity() {
    let (_writer, reader) = UnixStream::pair().unwrap();
    let cancelled = AtomicBool::new(true);
    assert_eq!(
        wait_for_pty_readiness(reader.as_raw_fd(), &cancelled, &inactive_probe()).unwrap(),
        PtyReadiness::Cancelled
    );
}

#[test]
fn readiness_wait_honors_a_quiescence_probe_without_descriptor_activity() {
    let (_writer, reader) = UnixStream::pair().unwrap();
    let probe = AtomicBool::new(true);
    assert_eq!(
        wait_for_pty_readiness(reader.as_raw_fd(), &active_wait(), &probe).unwrap(),
        PtyReadiness::Probe
    );
}

#[test]
fn readiness_wait_rechecks_cancellation_after_a_poll_timeout() {
    let (_writer, reader) = UnixStream::pair().unwrap();
    let cancelled = AtomicBool::new(false);

    assert_eq!(
        wait_for_pty_readiness_with(
            reader.as_raw_fd(),
            &cancelled,
            &inactive_probe(),
            timeout_then_cancel,
        )
        .unwrap(),
        PtyReadiness::Cancelled
    );
}

fn timeout_then_cancel(_descriptor: &mut libc::pollfd, cancelled: &AtomicBool) -> i32 {
    cancelled.store(true, std::sync::atomic::Ordering::Release);
    POLL_TIMEOUT_RESULT
}

#[test]
fn readiness_classification_drains_data_before_reporting_hangup() {
    assert_eq!(
        classify_poll_events(libc::POLLIN | libc::POLLHUP).unwrap(),
        PtyReadiness::Readable
    );
    assert_eq!(
        classify_poll_events(libc::POLLHUP).unwrap(),
        PtyReadiness::Closed
    );
    assert_eq!(
        classify_poll_events(libc::POLLERR).unwrap(),
        PtyReadiness::Closed
    );
    let invalid = classify_poll_events(libc::POLLNVAL).unwrap_err();
    assert!(invalid.to_string().contains(PTY_READER_POLL_INVALID_DETAIL));
    assert!(classify_poll_events(EMPTY_POLL_EVENTS).is_err());
}

#[test]
fn invalid_descriptor_cannot_enter_nonblocking_mode() {
    assert!(configure_nonblocking_fd(SYSCALL_FAILURE).is_err());
    let result = fcntl_set_flags(SYSCALL_FAILURE, libc::O_NONBLOCK, PTY_FD_NONBLOCK_CONTEXT);
    assert!(result.is_err());
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn interrupted_poll_retries_until_the_descriptor_is_ready() {
    let mut interrupted = false;
    let readiness = wait_for_pty_readiness_with(
        SYSCALL_FAILURE,
        &active_wait(),
        &inactive_probe(),
        |descriptor, _| {
            if !std::mem::replace(&mut interrupted, true) {
                set_errno(libc::EINTR);
                return SYSCALL_FAILURE;
            }
            descriptor.revents = libc::POLLIN;
            POLL_READY_DESCRIPTOR_COUNT
        },
    )
    .unwrap();

    assert_eq!(readiness, PtyReadiness::Readable);
    assert!(interrupted);
}

#[test]
fn non_interrupted_poll_failure_preserves_the_operating_system_error() {
    let error = wait_for_pty_readiness_with(
        SYSCALL_FAILURE,
        &active_wait(),
        &inactive_probe(),
        |_, _| {
            assert_eq!(unsafe { libc::close(SYSCALL_FAILURE) }, SYSCALL_FAILURE);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EBADF)
            );
            SYSCALL_FAILURE
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains(PTY_READER_POLL_CONTEXT));
}

#[test]
fn unexpected_poll_result_is_rejected() {
    let error = wait_for_pty_readiness_with(
        SYSCALL_FAILURE,
        &active_wait(),
        &inactive_probe(),
        |_, _| UNEXPECTED_POLL_RESULT,
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(PTY_READER_POLL_UNEXPECTED_DETAIL)
    );
}

#[cfg(target_os = "linux")]
fn set_errno(value: libc::c_int) {
    unsafe { *libc::__errno_location() = value };
}

#[cfg(target_os = "macos")]
fn set_errno(value: libc::c_int) {
    unsafe { *libc::__error() = value };
}

#[path = "nonblocking_pty_branch_matrix.rs"]
mod branch_matrix;
