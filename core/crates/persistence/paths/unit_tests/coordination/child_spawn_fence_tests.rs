use super::*;
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};

const INVALID_DESCRIPTOR: RawFd = libc::c_int::MAX;
const FIRST_NONSTANDARD_DESCRIPTOR: RawFd = libc::STDERR_FILENO + 1;

#[test]
fn standard_descriptors_are_rejected_without_kernel_inspection() {
    let error = validate_descriptor(libc::STDERR_FILENO).unwrap_err();
    assert!(error.to_string().contains(STANDARD_DESCRIPTOR_ERROR));
}

#[test]
fn invalid_descriptors_preserve_the_inspection_context() {
    let error = validate_descriptor(INVALID_DESCRIPTOR).unwrap_err();
    assert!(error.to_string().contains(DESCRIPTOR_INSPECTION_CONTEXT));
}

#[test]
fn inheritable_descriptors_are_rejected() {
    let _spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    let (source, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let descriptor = unsafe {
        libc::fcntl(
            source.as_raw_fd(),
            libc::F_DUPFD,
            FIRST_NONSTANDARD_DESCRIPTOR,
        )
    };
    assert!(descriptor > libc::STDERR_FILENO);
    let duplicate = unsafe { OwnedFd::from_raw_fd(descriptor) };

    let error = validate_descriptor(duplicate.as_raw_fd()).unwrap_err();

    assert!(error.to_string().contains(CLOSE_ON_EXEC_ERROR));
}

#[test]
fn close_on_exec_descriptors_are_accepted_and_can_be_closed() {
    let (source, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let descriptor = unsafe {
        libc::fcntl(
            source.as_raw_fd(),
            libc::F_DUPFD_CLOEXEC,
            FIRST_NONSTANDARD_DESCRIPTOR,
        )
    };
    assert!(descriptor > libc::STDERR_FILENO);
    let duplicate = unsafe { OwnedFd::from_raw_fd(descriptor) };
    validate_descriptor(duplicate.as_raw_fd()).unwrap();

    let descriptor = duplicate.into_raw_fd();
    close_descriptor(descriptor).unwrap();
    assert!(close_descriptor(INVALID_DESCRIPTOR).is_err());
}
