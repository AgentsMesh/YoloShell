use super::*;
use std::cell::RefCell;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const TEST_SOCKET_NAME: &str = "tested.sock";
const REPLACEMENT_BYTES: &[u8] = b"preserve-replacement";
const SOURCE_SOCKET_NAME: &str = "source.sock";
const INJECTED_SYNC_FAILURE: &str = "injected socket directory sync failure";
const INJECTED_QUARANTINE_FAILURE: &str = "injected quarantine failure";

struct Fixture {
    _root: tempfile::TempDir,
    directory: SocketDirectory,
    path: PathBuf,
    name: CString,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        let directory = SocketDirectory::new(PrivateDirectory::ensure(&path).unwrap());
        let name = CString::new(TEST_SOCKET_NAME).unwrap();
        Self {
            _root: root,
            directory,
            path,
            name,
        }
    }

    fn socket_path(&self) -> PathBuf {
        self.path.join(TEST_SOCKET_NAME)
    }

    fn bind(&self) -> std::os::unix::net::UnixListener {
        bind_private_socket(&self.socket_path())
    }

    fn identity(&self) -> SocketIdentity {
        self.directory.identity(&self.name).unwrap().unwrap()
    }
}

fn bind_private_socket(path: &Path) -> std::os::unix::net::UnixListener {
    let listener = std::os::unix::net::UnixListener::bind(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    listener
}

#[test]
fn expected_socket_is_quarantined_and_removed() {
    let fixture = Fixture::new();
    let listener = fixture.bind();
    let identity = fixture.identity();

    fixture
        .directory
        .remove_expected(&fixture.name, identity)
        .unwrap();

    assert!(!fixture.socket_path().exists());
    drop(listener);
}

#[test]
fn stale_identity_cannot_remove_a_socket_replacement() {
    let fixture = Fixture::new();
    let original = fixture.bind();
    let stale = fixture.identity();
    drop(original);
    fs::remove_file(fixture.socket_path()).unwrap();
    let replacement = fixture.bind();
    let expected = fixture.identity();

    assert!(
        fixture
            .directory
            .remove_expected(&fixture.name, stale)
            .is_err()
    );
    assert_eq!(fixture.identity(), expected);
    drop(replacement);
}

#[test]
fn invalid_replacement_is_restored_after_quarantine_inspection() {
    let fixture = Fixture::new();
    let original = fixture.bind();
    let stale = fixture.identity();
    drop(original);
    fs::remove_file(fixture.socket_path()).unwrap();
    fs::write(fixture.socket_path(), REPLACEMENT_BYTES).unwrap();

    assert!(
        fixture
            .directory
            .remove_expected(&fixture.name, stale)
            .is_err()
    );
    assert_eq!(fs::read(fixture.socket_path()).unwrap(), REPLACEMENT_BYTES);
}

#[test]
fn restore_collision_preserves_both_competing_entries() {
    let fixture = Fixture::new();
    let original = fixture.bind();
    let stale = fixture.identity();
    drop(original);
    fs::remove_file(fixture.socket_path()).unwrap();
    let replacement = fixture.bind();
    let replacement_identity = fixture.identity();
    let competitor = RefCell::new(None);

    let error = fixture
        .directory
        .remove_expected_with_hook(&fixture.name, stale, || {
            competitor.replace(Some(fixture.bind()));
            Ok(())
        })
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(BROKER_SOCKET_QUARANTINE_RESTORE_FAILED)
    );
    assert!(fixture.socket_path().exists());
    let quarantine = fs::read_dir(&fixture.path)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            entry
                .file_name()
                .as_bytes()
                .starts_with(BROKER_SOCKET_QUARANTINE_PREFIX.as_bytes())
        })
        .unwrap();
    let quarantine_name = CString::new(quarantine.file_name().as_bytes()).unwrap();
    assert_eq!(
        fixture.directory.identity(&quarantine_name).unwrap(),
        Some(replacement_identity)
    );
    drop(competitor);
    drop(replacement);
}

#[test]
fn quarantine_hook_failure_restores_the_expected_socket() {
    let fixture = Fixture::new();
    let listener = fixture.bind();
    let expected = fixture.identity();

    let error = fixture
        .directory
        .remove_expected_with_hook(&fixture.name, expected, || {
            anyhow::bail!(INJECTED_QUARANTINE_FAILURE)
        })
        .unwrap_err();

    assert!(error.to_string().contains(INJECTED_QUARANTINE_FAILURE));
    assert_eq!(fixture.identity(), expected);
    drop(listener);
}

#[test]
fn post_rename_sync_failure_restores_the_staging_socket() {
    let fixture = Fixture::new();
    let source_name = CString::new(SOURCE_SOCKET_NAME).unwrap();
    let source_path = fixture.path.join(SOURCE_SOCKET_NAME);
    let listener = bind_private_socket(&source_path);
    let expected = fixture.directory.identity(&source_name).unwrap().unwrap();

    let error = fixture
        .directory
        .publish_expected_from_with_commit(
            &fixture.directory,
            &source_name,
            &fixture.name,
            expected,
            || anyhow::bail!(INJECTED_SYNC_FAILURE),
        )
        .unwrap_err();

    assert!(error.to_string().contains(INJECTED_SYNC_FAILURE));
    assert_eq!(
        fixture.directory.identity(&source_name).unwrap(),
        Some(expected)
    );
    assert_eq!(fixture.directory.identity(&fixture.name).unwrap(), None);
    drop(listener);
}

#[path = "socket_directory_retry_tests.rs"]
mod retry_tests;
