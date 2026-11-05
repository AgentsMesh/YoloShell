use super::*;
use crate::paths::constants::PRIVATE_FILE_MODE;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener as StdListener, UnixStream as StdStream};
use tempfile::TempDir;

const DIRECTORY_PREFIX: &str = "socket-activation-";
const MANAGED_DIRECTORY: &str = "managed";
const DISPLACED_DIRECTORY: &str = "displaced";
const SOCKET_NAME: &str = "endpoint.sock";
const SENTINEL_NAME: &str = "sentinel";
const SENTINEL_VALUE: &[u8] = b"replacement";
const LIVE_ERROR: &str = "test endpoint is live";
const TEST_PROBE_TIMEOUT: Duration = Duration::from_millis(250);
const UNSAFE_SOCKET_MODE: u32 = 0o700;

struct Fixture {
    _root: TempDir,
    managed: PathBuf,
    socket: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let parent = Path::new(crate::paths::constants::SOCKET_ROOT_PREFIX)
            .parent()
            .unwrap();
        let root = tempfile::Builder::new()
            .prefix(DIRECTORY_PREFIX)
            .tempdir_in(parent)
            .unwrap();
        let managed = root.path().join(MANAGED_DIRECTORY);
        PrivateDirectory::ensure(&managed).unwrap();
        let socket = managed.join(SOCKET_NAME);
        Self {
            _root: root,
            managed,
            socket,
        }
    }

    fn parent(&self) -> PrivateDirectory {
        PrivateDirectory::open(&self.managed).unwrap()
    }

    fn secure_socket(&self) -> StdListener {
        let listener = StdListener::bind(&self.socket).unwrap();
        std::fs::set_permissions(
            &self.socket,
            std::fs::Permissions::from_mode(PRIVATE_FILE_MODE),
        )
        .unwrap();
        listener
    }
}

#[tokio::test]
async fn stale_socket_is_replaced_through_retained_parent() {
    let fixture = Fixture::new();
    drop(fixture.secure_socket());
    let (listener, publication) = publish(
        fixture.parent(),
        &fixture.socket,
        TEST_PROBE_TIMEOUT,
        LIVE_ERROR,
    )
    .await
    .unwrap();

    assert!(StdStream::connect(&fixture.socket).is_ok());
    drop(publication);
    assert!(!fixture.socket.exists());
    drop(listener);
}

#[tokio::test]
async fn live_socket_is_never_replaced() {
    let fixture = Fixture::new();
    let live = fixture.secure_socket();
    let error = publish(
        fixture.parent(),
        &fixture.socket,
        TEST_PROBE_TIMEOUT,
        LIVE_ERROR,
    )
    .await
    .err()
    .unwrap();

    assert!(error.to_string().contains(LIVE_ERROR));
    assert!(StdStream::connect(&fixture.socket).is_ok());
    drop(live);
}

#[tokio::test]
async fn unsafe_socket_mode_is_rejected_without_replacement() {
    use std::os::unix::fs::MetadataExt;

    let fixture = Fixture::new();
    let live = StdListener::bind(&fixture.socket).unwrap();
    std::fs::set_permissions(
        &fixture.socket,
        std::fs::Permissions::from_mode(UNSAFE_SOCKET_MODE),
    )
    .unwrap();
    let inode = std::fs::symlink_metadata(&fixture.socket).unwrap().ino();

    let error = publish(
        fixture.parent(),
        &fixture.socket,
        TEST_PROBE_TIMEOUT,
        LIVE_ERROR,
    )
    .await
    .err()
    .unwrap();

    assert!(format!("{error:#}").contains(SOCKET_MODE_ERROR));
    assert_eq!(
        std::fs::symlink_metadata(&fixture.socket).unwrap().ino(),
        inode
    );
    drop(live);
}

#[tokio::test]
async fn parent_swap_after_staging_never_publishes_in_replacement() {
    let fixture = Fixture::new();
    let displaced = fixture._root.path().join(DISPLACED_DIRECTORY);
    let target = SocketTarget::new(fixture.parent(), &fixture.socket).unwrap();
    let (listener, publication) = install_with_hook(target, || {
        std::fs::rename(&fixture.managed, &displaced)?;
        PrivateDirectory::ensure(&fixture.managed)?;
        std::fs::write(fixture.managed.join(SENTINEL_NAME), SENTINEL_VALUE)?;
        Ok(())
    })
    .await
    .unwrap();
    let displaced_socket = displaced.join(SOCKET_NAME);

    assert!(!fixture.socket.exists());
    assert!(StdStream::connect(&displaced_socket).is_ok());
    assert_eq!(
        std::fs::read(fixture.managed.join(SENTINEL_NAME)).unwrap(),
        SENTINEL_VALUE
    );
    drop(publication);
    assert!(!displaced_socket.exists());
    drop(listener);
}

#[tokio::test]
async fn cleanup_after_parent_swap_preserves_replacement_socket() {
    let fixture = Fixture::new();
    let (listener, publication) = publish(
        fixture.parent(),
        &fixture.socket,
        TEST_PROBE_TIMEOUT,
        LIVE_ERROR,
    )
    .await
    .unwrap();
    let displaced = fixture._root.path().join(DISPLACED_DIRECTORY);
    std::fs::rename(&fixture.managed, &displaced).unwrap();
    PrivateDirectory::ensure(&fixture.managed).unwrap();
    let replacement = fixture.secure_socket();

    drop(publication);
    assert!(!displaced.join(SOCKET_NAME).exists());
    assert!(StdStream::connect(&fixture.socket).is_ok());
    drop(replacement);
    drop(listener);
}
