use super::*;
use crate::paths::directory::PrivateDirectory;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const SHORT_SOCKET_TEMP_PARENT: &str = "/tmp";
const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;
const DISPLACED_DIRECTORY: &str = "state-displaced";
const TEST_SOCKET_NAME: &str = "tested.sock";
const SENTINEL_FILE: &str = "replacement-sentinel";
const SENTINEL_BYTES: &[u8] = b"preserve-replacement";
const DIRECTORY_CHANGED_MESSAGE: &str = "private directory path changed";
const INJECTED_RESTORE_FAILURE: &str = "injected thread directory restore failure";

struct Fixture {
    root: tempfile::TempDir,
    state: SocketDirectory,
    state_path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir_in(SHORT_SOCKET_TEMP_PARENT).unwrap();
        let state_path = root.path().join(STATE_DIRECTORY);
        let state = SocketDirectory::new(PrivateDirectory::ensure(&state_path).unwrap());
        Self {
            root,
            state,
            state_path,
        }
    }

    fn target_name() -> CString {
        CString::new(TEST_SOCKET_NAME).unwrap()
    }

    fn target_path(&self) -> PathBuf {
        self.state_path.join(TEST_SOCKET_NAME)
    }
}

fn bind_private_socket(path: &Path) -> std::os::unix::net::UnixListener {
    let listener = std::os::unix::net::UnixListener::bind(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    listener
}

#[tokio::test]
async fn cleanup_uses_retained_state_after_logical_replacement() {
    let fixture = Fixture::new();
    let bound = bind_private(&fixture.state, Fixture::target_name()).unwrap();
    let displaced = fixture.root.path().join(DISPLACED_DIRECTORY);
    fs::rename(&fixture.state_path, &displaced).unwrap();
    PrivateDirectory::ensure(&fixture.state_path).unwrap();
    let sentinel = fixture.state_path.join(SENTINEL_FILE);
    fs::write(&sentinel, SENTINEL_BYTES).unwrap();
    let replacement = bind_private_socket(&fixture.target_path());

    drop(bound);

    assert!(!displaced.join(TEST_SOCKET_NAME).exists());
    assert!(fixture.target_path().exists());
    assert_eq!(fs::read(sentinel).unwrap(), SENTINEL_BYTES);
    drop(replacement);
}

#[tokio::test]
async fn exclusive_publish_preserves_an_existing_target() {
    let fixture = Fixture::new();
    let replacement = bind_private_socket(&fixture.target_path());
    let expected = fixture.state.identity(&Fixture::target_name()).unwrap();

    assert!(bind_private(&fixture.state, Fixture::target_name()).is_err());
    assert_eq!(
        fixture.state.identity(&Fixture::target_name()).unwrap(),
        expected
    );
    drop(replacement);
}

#[tokio::test]
async fn bind_window_never_writes_the_replacement_state_root() {
    let fixture = Fixture::new();
    let state_path = fixture.state.path().to_owned();
    let displaced = fixture.root.path().join(DISPLACED_DIRECTORY);
    let sentinel = state_path.join(SENTINEL_FILE);

    let error = bind_private_with_hook(&fixture.state, Fixture::target_name(), || {
        fs::rename(&state_path, &displaced)?;
        PrivateDirectory::ensure(&state_path)?;
        fs::write(&sentinel, SENTINEL_BYTES)?;
        Ok(())
    })
    .err()
    .unwrap();

    assert!(error.to_string().contains(DIRECTORY_CHANGED_MESSAGE));
    assert_eq!(fs::read(&sentinel).unwrap(), SENTINEL_BYTES);
    assert_eq!(fs::read_dir(&state_path).unwrap().count(), 1);
    assert_eq!(fs::read_dir(displaced).unwrap().count(), 0);
    assert!(!fixture.target_path().exists());
}

#[tokio::test]
async fn restore_failure_removes_the_owned_staging_socket() {
    let fixture = Fixture::new();

    let error = bind_private_with_restore_failure(
        &fixture.state,
        Fixture::target_name(),
        INJECTED_RESTORE_FAILURE,
    )
    .err()
    .unwrap();

    assert!(format!("{error:#}").contains(INJECTED_RESTORE_FAILURE));
    assert_eq!(fs::read_dir(&fixture.state_path).unwrap().count(), 0);
    assert!(!fixture.target_path().exists());
}

#[tokio::test]
async fn restore_failure_preserves_a_replacement_staging_inode() {
    let fixture = Fixture::new();
    let mut replacement = None;
    let mut staging_path = None;

    let error = bind_private_with(
        &fixture.state,
        Fixture::target_name(),
        || Ok(()),
        |name, binding| {
            let path = fixture
                .state_path
                .join(std::ffi::OsStr::from_bytes(name.to_bytes()));
            fs::remove_file(&path).unwrap();
            replacement = Some(bind_private_socket(&path));
            staging_path = Some(path);
            binding.inject_restore_failure(INJECTED_RESTORE_FAILURE);
        },
    )
    .err()
    .unwrap();

    assert!(error.to_string().contains(BROKER_SOCKET_CHANGED));
    assert!(staging_path.unwrap().exists());
    assert_eq!(fs::read_dir(&fixture.state_path).unwrap().count(), 1);
    assert!(!fixture.target_path().exists());
    drop(replacement);
}

#[path = "socket_binding_retry_tests.rs"]
mod retry_tests;
