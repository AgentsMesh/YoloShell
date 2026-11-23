use super::*;
use std::ffi::CString;
use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;

const INVALID_DESCRIPTOR: RawFd = -1;
const MISSING_SOCKET_NAME: &str = "missing.sock";
const REGULAR_FILE_NAME: &str = "regular";
const SOCKET_NAME: &str = "identity.sock";
const REGULAR_BYTES: &[u8] = b"not-a-socket";
const PUBLIC_SOCKET_MODE: u32 = 0o666;
const TEST_DEVICE: libc::dev_t = 1;
const TEST_INODE: libc::ino_t = 2;

#[test]
fn stat_identity_rejects_type_owner_mode_and_invalid_device() {
    let mut metadata = socket_stat();
    metadata.st_mode = libc::S_IFREG as _;
    assert_eq!(
        identity_from_stat(&metadata, true).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );

    metadata = socket_stat();
    metadata.st_uid = foreign_uid();
    assert_eq!(
        identity_from_stat(&metadata, true).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );

    metadata = socket_stat();
    metadata.st_mode = (libc::S_IFSOCK as u32 | PUBLIC_SOCKET_MODE) as _;
    assert_eq!(
        identity_from_stat(&metadata, true).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert!(identity_from_stat(&metadata, false).is_ok());

    #[cfg(target_os = "macos")]
    {
        metadata = socket_stat();
        metadata.st_dev = -1;
        assert_eq!(
            identity_from_stat(&metadata, true).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }
}

#[test]
fn descriptor_identity_distinguishes_missing_invalid_and_non_socket_entries() {
    let root = tempfile::tempdir().unwrap();
    let directory = fs::File::open(root.path()).unwrap();
    let missing = CString::new(MISSING_SOCKET_NAME).unwrap();
    assert_eq!(
        socket_identity_at(directory.as_raw_fd(), &missing).unwrap(),
        None
    );
    assert_eq!(
        bound_socket_identity_at(directory.as_raw_fd(), &missing).unwrap(),
        None
    );
    assert!(socket_identity_at(INVALID_DESCRIPTOR, &missing).is_err());
    assert!(bound_socket_identity_at(INVALID_DESCRIPTOR, &missing).is_err());

    let regular = CString::new(REGULAR_FILE_NAME).unwrap();
    fs::write(root.path().join(REGULAR_FILE_NAME), REGULAR_BYTES).unwrap();
    assert!(socket_identity_at(directory.as_raw_fd(), &regular).is_err());
    assert!(bound_socket_identity_at(directory.as_raw_fd(), &regular).is_err());
}

#[test]
fn path_and_descriptor_identity_apply_distinct_permission_policies() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(SOCKET_NAME);
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(PUBLIC_SOCKET_MODE)).unwrap();
    let directory = fs::File::open(root.path()).unwrap();
    let name = CString::new(SOCKET_NAME).unwrap();

    assert!(socket_identity(&path).is_err());
    assert!(socket_identity_at(directory.as_raw_fd(), &name).is_err());
    assert!(
        bound_socket_identity_at(directory.as_raw_fd(), &name)
            .unwrap()
            .is_some()
    );

    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    assert_eq!(
        socket_identity(&path).unwrap(),
        socket_identity_at(directory.as_raw_fd(), &name)
            .unwrap()
            .unwrap()
    );
    drop(listener);
}

fn socket_stat() -> libc::stat {
    let mut metadata = unsafe { std::mem::zeroed::<libc::stat>() };
    metadata.st_mode = (libc::S_IFSOCK as u32 | PRIVATE_FILE_MODE) as _;
    metadata.st_uid = unsafe { libc::geteuid() };
    metadata.st_dev = TEST_DEVICE;
    metadata.st_ino = TEST_INODE;
    metadata
}

fn foreign_uid() -> libc::uid_t {
    let current = unsafe { libc::geteuid() };
    let candidate = current.wrapping_add(1);
    assert_ne!(candidate, current);
    candidate
}
