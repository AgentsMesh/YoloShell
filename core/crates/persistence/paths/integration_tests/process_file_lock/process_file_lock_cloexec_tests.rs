use super::*;
use crate::paths::constants::PRIVATE_FILE_MODE;
use crate::paths::identity::ObjectIdentity;
use std::fs::{File, OpenOptions};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

const LOCK_FILE_NAME: &str = "cloexec.lock";
const KERNEL_LEASE_EXPECTED_DETAIL: &str = "kernel lease descriptor expected";
const FCNTL_FAILURE: libc::c_int = -1;
const FCNTL_SUCCESS: libc::c_int = 0;
const FD_FLAG_UNSET: libc::c_int = 0;

#[test]
fn cloned_lease_descriptor_is_close_on_exec() {
    let fixture = Fixture::new();
    let source = open_file(&fixture.path);
    clear_close_on_exec(&source);
    let identity = ObjectIdentity::from_metadata(&source.metadata().unwrap());

    let lock = ProcessFileLock::try_lock(&source, identity, Mode::Shared)
        .unwrap()
        .unwrap();

    assert_eq!(
        descriptor_flags(source.as_raw_fd()) & libc::FD_CLOEXEC,
        FD_FLAG_UNSET
    );
    assert_ne!(
        descriptor_flags(lease_descriptor(&lock)) & libc::FD_CLOEXEC,
        FD_FLAG_UNSET
    );
}

fn lease_descriptor(lock: &ProcessFileLock) -> RawFd {
    match &lock.lease {
        LockLease::Kernel(lease) => lease.descriptor.as_raw_fd(),
        LockLease::Logical => panic!("{KERNEL_LEASE_EXPECTED_DETAIL}"),
    }
}

fn clear_close_on_exec(file: &File) {
    let flags = descriptor_flags(file.as_raw_fd());
    assert_eq!(
        unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, flags & !libc::FD_CLOEXEC) },
        FCNTL_SUCCESS
    );
}

fn descriptor_flags(descriptor: RawFd) -> libc::c_int {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    flags
}

struct Fixture {
    _temporary: tempfile::TempDir,
    path: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join(LOCK_FILE_NAME);
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(PRIVATE_FILE_MODE)
            .open(&path)
            .unwrap();
        Self {
            _temporary: temporary,
            path,
        }
    }
}

fn open_file(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap()
}
