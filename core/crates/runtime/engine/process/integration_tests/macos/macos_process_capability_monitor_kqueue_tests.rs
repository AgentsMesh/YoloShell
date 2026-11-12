use super::*;

#[test]
fn system_failure_preserves_errno_and_context() {
    // SAFETY: closing an invalid descriptor cannot close a process-owned resource.
    let result = unsafe { libc::close(INVALID_SYSTEM_RESULT) };
    let error = checked_system_result(result, QUEUE_READ_CONTEXT).unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::raw_os_error),
        Some(libc::EBADF)
    );
    assert!(format!("{error:#}").contains(QUEUE_READ_CONTEXT));
}
