use super::*;
use crate::unit_support::*;
use std::os::fd::AsRawFd;

#[test]
fn descriptor_flags_are_added_without_removing_existing_flags() {
    let file = tempfile::tempfile().unwrap();
    let descriptor = file.as_raw_fd();
    let before = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    set_close_on_exec(descriptor).unwrap();
    set_nonblocking(descriptor).unwrap();
    assert_ne!(
        unsafe { libc::fcntl(descriptor, libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
    let after = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    assert_eq!(after, before | libc::O_NONBLOCK);
    set_nonblocking(descriptor).unwrap();
}

#[test]
fn invalid_get_and_set_operations_report_the_underlying_errno() {
    for result in [
        set_close_on_exec(INVALID_DESCRIPTOR),
        set_nonblocking(INVALID_DESCRIPTOR),
    ] {
        assert_eq!(
            result
                .unwrap_err()
                .downcast_ref::<std::io::Error>()
                .unwrap()
                .raw_os_error(),
            Some(libc::EBADF)
        );
    }
    let file = tempfile::tempfile().unwrap();
    let error = set_flags(
        file.as_raw_fd(),
        libc::F_GETFD,
        INVALID_DESCRIPTOR,
        libc::FD_CLOEXEC,
        DESCRIPTION,
    )
    .unwrap_err();
    assert!(error.downcast_ref::<std::io::Error>().is_some());
    assert!(error.to_string().contains(DESCRIPTION));
}
