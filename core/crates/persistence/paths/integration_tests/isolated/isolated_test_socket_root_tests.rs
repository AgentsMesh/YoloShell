use super::*;
use std::fs::{self, File};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};

const UNKNOWN_ENTRY_NAME: &str = "unexpected-entry";
const REPLACEMENT_ENTRY_NAME: &str = "replacement-entry";
const DISPLACED_ROOT_PREFIX: &str = "yd-";
const EARLY_RETURN_DETAIL: &str = "injected return after isolated root creation";

#[test]
fn isolated_test_socket_root_is_private_bounded_and_finalizable() {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let path = root.path().to_path_buf();
    let metadata = path.metadata().unwrap();
    let directory = File::open(&path).unwrap();

    assert!(path.as_os_str().as_bytes().len() <= ISOLATED_TEST_SOCKET_ROOT_MAX_BYTES);
    assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
    assert_eq!(
        metadata.mode() & PERMISSION_MODE_MASK,
        PRIVATE_DIRECTORY_MODE
    );
    crate::private_acl::require_empty(&directory, &path).unwrap();

    root.finalize().unwrap();
    assert!(!path.exists());
    root.finalize().unwrap();
}

#[test]
fn isolated_test_socket_root_preserves_unknown_entries() {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let path = root.path().to_path_buf();
    let marker = path.join(UNKNOWN_ENTRY_NAME);
    File::create(&marker).unwrap();

    let rejected = root.finalize().is_err();
    let preserved = path.is_dir() && marker.is_file();
    fs::remove_file(&marker).unwrap();
    root.finalize().unwrap();

    assert!(rejected);
    assert!(preserved);
    assert!(!path.exists());
}

#[test]
fn isolated_test_socket_root_preserves_a_busy_lifecycle_lock() {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let path = root.path().to_path_buf();
    let lifecycle = root.hold_lifecycle_for_test().unwrap();

    let rejected = root.finalize().is_err();
    let preserved = path.is_dir();
    drop(lifecycle);
    root.finalize().unwrap();

    assert!(rejected);
    assert!(preserved);
    assert!(!path.exists());
}

#[test]
fn isolated_test_socket_root_does_not_remove_a_replacement() {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let path = root.path().to_path_buf();
    let displaced = path.with_file_name(format!(
        "{DISPLACED_ROOT_PREFIX}{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::rename(&path, &displaced).unwrap();
    fs::DirBuilder::new()
        .mode(PRIVATE_DIRECTORY_MODE)
        .create(&path)
        .unwrap();
    let marker = path.join(REPLACEMENT_ENTRY_NAME);
    File::create(&marker).unwrap();

    let rejected = root.finalize().is_err();
    let replacement_preserved = path.is_dir() && marker.is_file();
    fs::remove_file(&marker).unwrap();
    fs::remove_dir(&path).unwrap();
    fs::rename(&displaced, &path).unwrap();
    root.finalize().unwrap();

    assert!(rejected);
    assert!(replacement_preserved);
    assert!(!path.exists());
}

#[test]
fn isolated_test_socket_root_drop_finalizes_after_early_return() {
    let mut path = None;
    let error = return_after_isolated_root_creation(&mut path).unwrap_err();
    assert!(error.to_string().contains(EARLY_RETURN_DETAIL));
    assert!(!path.unwrap().exists());
}

fn return_after_isolated_root_creation(path: &mut Option<PathBuf>) -> Result<()> {
    let root = IsolatedTestSocketRoot::create()?;
    *path = Some(root.path().to_path_buf());
    anyhow::bail!(EARLY_RETURN_DETAIL)
}
