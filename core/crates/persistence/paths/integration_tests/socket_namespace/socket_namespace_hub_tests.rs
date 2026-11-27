use super::*;
use std::time::Duration;

const LIVE_ERROR: &str = "Hub endpoint unexpectedly live";
const PROBE_TIMEOUT: Duration = Duration::from_millis(250);

#[tokio::test]
async fn hub_endpoint_survives_another_namespace_holders_drop_and_retires_with_its_owner() {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::new(root.path());
    let first = paths.retain_socket_namespace().unwrap();
    let owner = paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap().path().to_path_buf();
    let socket = owner.hub_socket().unwrap();
    let (listener, publication) = crate::paths::socket_activation::publish(
        owner.directory().unwrap(),
        &socket,
        PROBE_TIMEOUT,
        LIVE_ERROR,
    )
    .await
    .unwrap();
    drop(first);
    owner.verify().unwrap();
    let client = tokio::net::UnixStream::connect(&socket).await.unwrap();
    let accepted = listener.accept().await.unwrap();
    drop(accepted);
    drop(client);
    drop(listener);
    publication.withdraw().unwrap();
    drop(owner);
    assert!(!directory.exists());
}

#[tokio::test]
async fn last_owner_reclaims_proven_stale_hub_and_preserves_successor_publication() {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::new(root.path());
    let owner = paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap().path().to_path_buf();
    let socket = owner.hub_socket().unwrap();
    let (listener, publication) = crate::paths::socket_activation::publish(
        owner.directory().unwrap(),
        &socket,
        PROBE_TIMEOUT,
        LIVE_ERROR,
    )
    .await
    .unwrap();
    let mut publication = std::mem::ManuallyDrop::new(publication);
    drop(listener);
    drop(owner);
    assert!(!directory.exists());
    assert!(root.path().is_dir());
    let recovered = paths.retain_socket_namespace().unwrap();
    let (listener, current) = crate::paths::socket_activation::publish(
        recovered.directory().unwrap(),
        &socket,
        PROBE_TIMEOUT,
        LIVE_ERROR,
    )
    .await
    .unwrap();
    unsafe {
        std::mem::ManuallyDrop::drop(&mut publication);
    }
    assert!(tokio::net::UnixStream::connect(&socket).await.is_ok());
    drop(listener);
    current.withdraw().unwrap();
    drop(recovered);
    assert!(!directory.exists());
}
