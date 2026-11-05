use super::{btrfs_uuid, capture, generic_uuid};
use crate::{DurableObjectIdentity, Error};
use std::{
    fs::{File, OpenOptions},
    os::{fd::AsFd, unix::fs::OpenOptionsExt},
};

const PROC_DIRECTORY: &str = "/proc/self";
const SHARED_MEMORY_DIRECTORY: &str = "/dev/shm";

#[test]
fn unsupported_filesystem_and_uuid_interfaces_do_not_fabricate_identity() {
    let proc = File::open(PROC_DIRECTORY).unwrap();
    assert!(matches!(capture(proc.as_fd()), Err(Error::Unsupported(_))));
    assert!(matches!(
        btrfs_uuid(proc.as_fd()),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        generic_uuid(proc.as_fd()),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn path_only_descriptor_is_not_io_authority_for_filesystem_uuid() {
    let directory = tempfile::tempdir().unwrap();
    let descriptor = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH)
        .open(directory.path())
        .unwrap();
    for result in [
        btrfs_uuid(descriptor.as_fd()),
        generic_uuid(descriptor.as_fd()),
    ] {
        match result {
            Err(Error::Io { source, .. }) => assert_eq!(source.raw_os_error(), Some(libc::EBADF)),
            other => panic!("path-only descriptor must not authorize UUID ioctl: {other:?}"),
        }
    }
}

#[test]
fn tmpfs_instance_has_a_generic_uuid_and_export_handle() {
    let directory = tempfile::tempdir_in(SHARED_MEMORY_DIRECTORY).unwrap();
    let first = File::open(directory.path()).unwrap();
    let proof = DurableObjectIdentity::capture(first.as_fd()).unwrap();
    let uuid = generic_uuid(first.as_fd()).unwrap();
    assert_ne!(uuid, [u8::default(); super::UUID_BYTES]);
    proof
        .verify(File::open(directory.path()).unwrap().as_fd())
        .unwrap();
    assert!(matches!(
        btrfs_uuid(first.as_fd()),
        Err(Error::Unsupported(_))
    ));
}
