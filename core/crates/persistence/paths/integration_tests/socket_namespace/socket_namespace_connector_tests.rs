use super::{test_support::*, *};
use std::{
    fs,
    os::unix::{fs::PermissionsExt, net::UnixListener},
    time::Duration,
};

const UNKNOWN_SOCKET_NAME: &str = "connector-extra.sock";
const PRESERVED_BYTES: &[u8] = b"preserve";
const PROBE_TIMEOUT: Duration = Duration::from_millis(250);
const LIVE_ERROR: &str = "connector endpoint unexpectedly live";

#[tokio::test]
async fn published_connector_survives_peer_release_and_namespace_scanning() {
    let fixture = RegisteredFixture::new();
    let owner = fixture.paths.retain_socket_namespace().unwrap();
    let other = fixture.paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap();
    let socket = owner.connector_socket().unwrap();
    let (listener, publication) = crate::paths::socket_activation::publish(
        directory.try_clone().unwrap(),
        &socket,
        PROBE_TIMEOUT,
        LIVE_ERROR,
    )
    .await
    .unwrap();
    assert_eq!(other.release().unwrap(), NamespaceRelease::Busy);
    run_scanner_pass(&owner);
    owner.verify().unwrap();
    tokio::time::timeout(PROBE_TIMEOUT, async {
        let client = tokio::net::UnixStream::connect(&socket).await.unwrap();
        let accepted = listener.accept().await.unwrap();
        drop((client, accepted));
    })
    .await
    .unwrap();
    drop(listener);
    publication.withdraw().unwrap();
    assert_eq!(owner.release().unwrap(), NamespaceRelease::Removed);
    assert!(!directory.path().exists());
}

#[test]
fn connector_lookalikes_and_exact_named_regular_files_are_preserved() {
    for regular_file in [false, true] {
        let fixture = RegisteredFixture::new();
        let owner = fixture.paths.retain_socket_namespace().unwrap();
        let directory = owner.directory().unwrap();
        let name = if regular_file {
            CONNECTOR_SOCKET_FILE_NAME
        } else {
            UNKNOWN_SOCKET_NAME
        };
        let path = directory.path().join(name);
        if regular_file {
            fs::write(&path, PRESERVED_BYTES).unwrap();
        } else {
            drop(UnixListener::bind(&path).unwrap());
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        run_scanner_pass(&owner);
        let release = owner.release();
        if regular_file {
            assert!(release.is_err());
        } else {
            assert_eq!(release.unwrap(), NamespaceRelease::Preserved);
        }
        assert!(path.exists());
        if regular_file {
            assert_eq!(fs::read(&path).unwrap(), PRESERVED_BYTES);
        }
    }
}
