use super::{filesystem, handle};
use crate::{DurableObjectIdentity, Error};
use std::{
    fs::File,
    os::fd::{AsFd, AsRawFd},
    process::Command,
};

#[path = "../test_support/syscall_constants.rs"]
mod constants;
#[path = "../test_support/seccomp.rs"]
mod seccomp;
use constants::{EXACT_ARGUMENT, NOCAPTURE_ARGUMENT};

const CHILD_TEST: &str = "platform::linux::syscall_tests::syscall_denial_child";
const DENIAL_ENV: &str = "YOLO_DURABLE_IDENTITY_DENIED_SYSCALL";
const STATX: &str = "statx";
const STATFS: &str = "fstatfs";
const HANDLE_DENIED: &str = "handle-denied";
const HANDLE_UNSUPPORTED: &str = "handle-unsupported";
const HANDLE_OVERFLOW: &str = "handle-overflow";
const IOCTL: &str = "ioctl";
const CHILD_SUCCESS: &str = "kernel denial correctly rejected";

#[test]
fn denied_kernel_operations_fail_closed_without_invalidating_descriptors() {
    for denial in [
        STATX,
        STATFS,
        HANDLE_DENIED,
        HANDLE_UNSUPPORTED,
        HANDLE_OVERFLOW,
        IOCTL,
    ] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([EXACT_ARGUMENT, CHILD_TEST, NOCAPTURE_ARGUMENT])
            .env(DENIAL_ENV, denial)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{denial}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains(CHILD_SUCCESS));
    }
}

#[test]
fn syscall_denial_child() {
    let Some(denial) = std::env::var_os(DENIAL_ENV) else {
        return;
    };
    let directory = tempfile::tempdir().unwrap();
    let file = File::open(directory.path()).unwrap();
    let (syscall, errno) = match denial.to_str().unwrap() {
        STATX => (libc::SYS_statx, libc::EACCES),
        STATFS => (libc::SYS_fstatfs, libc::EACCES),
        HANDLE_DENIED => (libc::SYS_name_to_handle_at, libc::EACCES),
        HANDLE_UNSUPPORTED => (libc::SYS_name_to_handle_at, libc::ENOSYS),
        HANDLE_OVERFLOW => (libc::SYS_name_to_handle_at, libc::EOVERFLOW),
        IOCTL => (libc::SYS_ioctl, libc::EACCES),
        _ => panic!("invalid kernel denial case"),
    };
    seccomp::deny(syscall, errno).unwrap();
    let error = match denial.to_str().unwrap() {
        STATX => DurableObjectIdentity::capture(file.as_fd()).err().unwrap(),
        STATFS => filesystem::capture(file.as_fd()).err().unwrap(),
        IOCTL => filesystem::capture(file.as_fd()).err().unwrap(),
        _ => handle::capture(file.as_fd()).err().unwrap(),
    };
    if matches!(errno, libc::ENOSYS | libc::EOVERFLOW) {
        assert!(matches!(error, Error::Unsupported(_)));
    } else {
        match error {
            Error::Io { source, .. } => assert_eq!(source.raw_os_error(), Some(errno)),
            other => panic!("expected kernel permission denial, got {other:?}"),
        }
    }
    // SAFETY: F_GETFD is read-only and this File still owns the valid descriptor.
    assert!(unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } >= libc::c_int::default());
    println!("{CHILD_SUCCESS}");
}
