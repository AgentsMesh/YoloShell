use super::{btrfs_uuid, capture, ext4_uuid, generic_uuid};
use crate::{DurableObjectIdentity, Error};
use std::{
    fs::{File, OpenOptions},
    os::{
        fd::{AsFd, AsRawFd},
        unix::fs::OpenOptionsExt,
    },
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
        ext4_uuid(descriptor.as_fd()),
    ] {
        match result {
            Err(Error::Io { source, .. }) => assert_eq!(source.raw_os_error(), Some(libc::EBADF)),
            other => panic!("path-only descriptor must not authorize UUID ioctl: {other:?}"),
        }
    }
}

#[test]
fn ext4_native_uuid_matches_the_persisted_identity_of_files_and_directories() {
    let directory = tempfile::tempdir().unwrap();
    let regular = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
    for path in [directory.path(), regular.path()] {
        let file = File::open(path).unwrap();
        let (kind, uuid) = capture(file.as_fd()).unwrap();
        if kind != crate::model::EXT4_MAGIC {
            return;
        }
        assert_eq!(ext4_uuid(file.as_fd()).unwrap(), uuid);
        let identity = DurableObjectIdentity::capture(file.as_fd()).unwrap();
        identity.verify(File::open(path).unwrap().as_fd()).unwrap();
    }
}

#[test]
fn tmpfs_identity_follows_the_running_kernels_uuid_support() {
    let directory = tempfile::tempdir_in(SHARED_MEMORY_DIRECTORY).unwrap();
    let first = File::open(directory.path()).unwrap();
    let Some(expected) = kernel_filesystem_uuid(&first) else {
        assert!(matches!(
            DurableObjectIdentity::capture(first.as_fd()),
            Err(Error::Unsupported("filesystem UUID"))
        ));
        return;
    };
    let proof = DurableObjectIdentity::capture(first.as_fd()).unwrap();
    let uuid = generic_uuid(first.as_fd()).unwrap();
    assert_eq!(uuid, expected);
    assert_ne!(uuid, [u8::default(); super::UUID_BYTES]);
    proof
        .verify(File::open(directory.path()).unwrap().as_fd())
        .unwrap();
    assert!(matches!(
        btrfs_uuid(first.as_fd()),
        Err(Error::Unsupported(_))
    ));
}

// Check the actual Linux UAPI independently of the production UUID reader.
fn kernel_filesystem_uuid(file: &File) -> Option<[u8; super::UUID_BYTES]> {
    const BUFFER_BYTES: usize = 1 + super::UUID_BYTES;
    let mut buffer = [0u8; BUFFER_BYTES];
    let request = libc::_IOR::<[u8; BUFFER_BYTES]>(0x15, 0);
    // SAFETY: the buffer holds the UAPI length byte and all sixteen UUID bytes.
    if unsafe { libc::ioctl(file.as_raw_fd(), request, buffer.as_mut_ptr()) } == -1 {
        let error = std::io::Error::last_os_error();
        assert!(
            matches!(
                error.raw_os_error(),
                Some(libc::ENOTTY | libc::EOPNOTSUPP | libc::ENOSYS)
            ),
            "{error}"
        );
        return None;
    }
    assert_eq!(usize::from(buffer[0]), super::UUID_BYTES);
    Some(buffer[1..].try_into().unwrap())
}
