use super::*;
use std::ffi::CString;
use std::fs;
use std::os::fd::AsRawFd;

const INVALID_DESCRIPTOR: RawFd = -1;
const SOCKET_NAME: &str = "relative.sock";
const NESTED_SOCKET_NAME: &str = "nested/relative.sock";
const ROOT_FALLBACK: &str = "/";
const EXISTING_BIND_SUCCEEDED_DETAIL: &str = "existing relative socket bind succeeded";
const NESTED_BIND_SUCCEEDED_DETAIL: &str = "nested relative socket bind succeeded";
const ORIGINAL_DIRECTORY: &str = "original";
const RETAINED_DIRECTORY: &str = "retained";
const REGULAR_FILE: &str = "regular";

#[test]
fn relative_bind_rejects_invalid_directory_and_existing_name() {
    let root = tempfile::tempdir().unwrap();
    let directory = fs::File::open(root.path()).unwrap();
    let name = CString::new(SOCKET_NAME).unwrap();

    assert!(bind(INVALID_DESCRIPTOR, root.path(), &name).is_err());
    assert!(!root.path().join(SOCKET_NAME).exists());
    let existing = std::os::unix::net::UnixListener::bind(root.path().join(SOCKET_NAME)).unwrap();
    let error = match bind(directory.as_raw_fd(), root.path(), &name) {
        Ok(_) => panic!("{EXISTING_BIND_SUCCEEDED_DETAIL}"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
    drop(existing);
}

#[test]
fn relative_bind_uses_the_retained_directory_after_path_replacement() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join(ORIGINAL_DIRECTORY);
    let retained = root.path().join(RETAINED_DIRECTORY);
    fs::create_dir(&original).unwrap();
    let directory = fs::File::open(&original).unwrap();
    fs::rename(&original, &retained).unwrap();
    fs::create_dir(&original).unwrap();
    let name = CString::new(SOCKET_NAME).unwrap();
    let binding = bind(directory.as_raw_fd(), &original, &name).unwrap();
    assert!(retained.join(SOCKET_NAME).exists());
    assert!(!original.join(SOCKET_NAME).exists());
    assert_eq!(
        bound_socket_identity_at(directory.as_raw_fd(), &name).unwrap(),
        Some(binding.identity)
    );
}

#[test]
fn relative_bind_rejects_a_regular_descriptor_without_creating_a_socket() {
    let root = tempfile::tempdir().unwrap();
    let file = fs::File::create(root.path().join(REGULAR_FILE)).unwrap();
    let name = CString::new(SOCKET_NAME).unwrap();
    assert!(bind(file.as_raw_fd(), root.path(), &name).is_err());
    assert!(!root.path().join(SOCKET_NAME).exists());
}

#[test]
fn relative_bind_rejects_a_path_separator_before_touching_the_directory() {
    let nested = CString::new(NESTED_SOCKET_NAME).unwrap();

    let error = match bind(
        INVALID_DESCRIPTOR,
        std::path::Path::new(ROOT_FALLBACK),
        &nested,
    ) {
        Ok(_) => panic!("{NESTED_BIND_SUCCEEDED_DETAIL}"),
        Err(error) => error,
    };

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn relative_bind_returns_identity_and_listener_custody() {
    let root = tempfile::tempdir().unwrap();
    let directory = fs::File::open(root.path()).unwrap();
    let name = CString::new(SOCKET_NAME).unwrap();

    let binding = bind(directory.as_raw_fd(), root.path(), &name).unwrap();
    assert_eq!(
        bound_socket_identity_at(directory.as_raw_fd(), &name)
            .unwrap()
            .unwrap(),
        binding.identity
    );
    assert!(binding.restore_error.is_none());
    drop(binding.listener);
}

#[cfg(target_os = "macos")]
#[test]
fn thread_directory_drop_restores_an_uncommitted_scope() {
    let root = tempfile::tempdir().unwrap();
    let directory = fs::File::open(root.path()).unwrap();
    let entered = ThreadDirectory::enter(directory.as_raw_fd()).unwrap();
    assert!(entered.active);
    drop(entered);

    let restored = ThreadDirectory::enter(directory.as_raw_fd()).unwrap();
    restored.restore().unwrap();
}
