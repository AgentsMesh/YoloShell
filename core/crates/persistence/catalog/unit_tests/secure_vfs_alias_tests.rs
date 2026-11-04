use super::secure_vfs_file::ArtifactKind;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::Path;
use std::sync::{Arc, Mutex};

const EMPTY_SQLITE_BYTES: &[u8] = b"";
const EXTERNAL_ALIAS_SENTINEL: &[u8] = b"external alias sentinel";
const PERMISSION_MODE_MASK: u32 = 0o777;
const DEAD_PROCESS_ID: libc::pid_t = libc::pid_t::MAX;

#[test]
fn exclusive_secure_open_rejects_an_existing_artifact() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("existing.sqlite");
    std::fs::write(&path, EMPTY_SQLITE_BYTES).unwrap();
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    let flags = rusqlite::ffi::SQLITE_OPEN_MAIN_DB
        | rusqlite::ffi::SQLITE_OPEN_READWRITE
        | rusqlite::ffi::SQLITE_OPEN_CREATE
        | rusqlite::ffi::SQLITE_OPEN_EXCLUSIVE;

    let error = super::secure_vfs_file::SecureDescriptor::open(&name, flags)
        .err()
        .unwrap();

    assert!(error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|error| error.kind() == std::io::ErrorKind::AlreadyExists)
    }));
}

#[test]
fn alias_is_private_and_on_the_artifact_filesystem() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite");
    std::fs::write(&path, EMPTY_SQLITE_BYTES).unwrap();
    let descriptor = descriptor(&path);
    let alias =
        super::secure_vfs_alias::SecureAlias::create(&descriptor, ArtifactKind::Main).unwrap();
    let alias_metadata = std::fs::symlink_metadata(alias.directory()).unwrap();
    let artifact_metadata = std::fs::symlink_metadata(&path).unwrap();

    assert_eq!(alias.directory().parent(), path.parent());
    assert_eq!(alias_metadata.dev(), artifact_metadata.dev());
    assert_eq!(
        alias_metadata.mode() & PERMISSION_MODE_MASK,
        super::secure_vfs_alias_io::ALIAS_ACTIVE_MODE
    );
}

#[test]
fn cleanup_does_not_remove_a_published_alias_with_an_active_lock() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite");
    std::fs::write(&path, EMPTY_SQLITE_BYTES).unwrap();
    let descriptor = descriptor(&path);
    let alias =
        super::secure_vfs_alias::SecureAlias::create(&descriptor, ArtifactKind::Main).unwrap();
    let alias_path = alias.directory().to_path_buf();

    super::secure_vfs_cleanup::cleanup_stale_aliases(&path).unwrap();

    assert!(alias_path.is_dir());
    assert_eq!(std::fs::symlink_metadata(path).unwrap().nlink(), 2);
}

#[test]
fn stale_published_alias_releases_a_crash_left_hard_link() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("catalog.sqlite");
    std::fs::write(&artifact, EMPTY_SQLITE_BYTES).unwrap();
    let stale = directory.path().join(format!(
        "{}{}",
        super::secure_vfs_alias_io::ALIAS_PREFIX,
        uuid::Uuid::new_v4()
    ));
    private_directory(&stale);
    let directory = super::secure_vfs_alias_directory::open_verified(&stale).unwrap();
    let lock = super::secure_vfs_alias_io::create_lock_file(&directory, &stale).unwrap();
    std::fs::hard_link(
        &artifact,
        stale.join(super::secure_vfs_alias_io::ALIAS_MAIN_NAME),
    )
    .unwrap();
    drop(lock);

    super::secure_vfs_cleanup::cleanup_stale_aliases(&artifact).unwrap();

    assert!(!stale.exists());
    assert_eq!(std::fs::symlink_metadata(artifact).unwrap().nlink(), 1);
}

#[test]
fn live_initialization_without_a_lock_is_not_cleaned() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("catalog.sqlite");
    let initializing = initializing_path(directory.path(), std::process::id() as libc::pid_t);
    private_directory(&initializing);

    super::secure_vfs_cleanup::cleanup_stale_aliases(&artifact).unwrap();

    assert!(initializing.is_dir());
}

#[test]
fn dead_initialization_without_a_lock_is_cleaned() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("catalog.sqlite");
    let initializing = initializing_path(directory.path(), DEAD_PROCESS_ID);
    private_directory(&initializing);

    super::secure_vfs_cleanup::cleanup_stale_aliases(&artifact).unwrap();

    assert!(!initializing.exists());
}

#[test]
fn main_alias_is_sealed_while_the_base_vfs_opens_it() {
    assert_alias_is_sealed(ArtifactKind::Main);
}

#[test]
fn shm_alias_is_sealed_while_the_base_vfs_opens_it() {
    assert_alias_is_sealed(ArtifactKind::Shm);
}

fn assert_alias_is_sealed(kind: ArtifactKind) {
    let directory = tempfile::tempdir().unwrap();
    let catalog = directory.path().join("catalog.sqlite");
    let outside = directory.path().join("outside.sqlite");
    std::fs::write(&outside, EXTERNAL_ALIAS_SENTINEL).unwrap();
    let removal_error = Arc::new(Mutex::new(None));
    let observed = Arc::clone(&removal_error);
    super::secure_vfs_tests::set_alias(kind, move |alias| {
        let result = std::fs::remove_file(alias);
        *observed.lock().unwrap() = result.err().map(|error| error.kind());
        Ok(())
    });

    let catalog_result = super::Catalog::open(&catalog);

    assert!(catalog_result.is_ok());
    assert_eq!(
        *removal_error.lock().unwrap(),
        Some(std::io::ErrorKind::PermissionDenied)
    );
    assert_eq!(std::fs::read(outside).unwrap(), EXTERNAL_ALIAS_SENTINEL);
}

fn private_directory(path: &Path) {
    let mut builder = std::fs::DirBuilder::new();
    builder
        .mode(super::secure_vfs_alias_io::ALIAS_ACTIVE_MODE)
        .create(path)
        .unwrap();
}

fn initializing_path(parent: &Path, pid: libc::pid_t) -> std::path::PathBuf {
    parent.join(format!(
        "{}{}-{}",
        super::secure_vfs_alias_io::ALIAS_INITIALIZING_PREFIX,
        pid,
        uuid::Uuid::new_v4()
    ))
}

fn descriptor(path: &Path) -> super::secure_vfs_file::SecureDescriptor {
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    let flags = rusqlite::ffi::SQLITE_OPEN_MAIN_DB | rusqlite::ffi::SQLITE_OPEN_READWRITE;
    super::secure_vfs_file::SecureDescriptor::open(&name, flags).unwrap()
}
