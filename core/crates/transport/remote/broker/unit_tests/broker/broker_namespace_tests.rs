use super::super::constants::{
    BROKER_NAMESPACE_LOST_CONTEXT, MANAGEMENT_SOCKET_FILE, RUNTIME_SOCKET_FILE,
};
use super::super::sockets::{BrokerSocketBindStage, BrokerSockets};
use super::super::*;
use super::server_support::Harness;
use super::support::FakeFactory;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tokio::net::UnixListener;

const DISPLACED_DIRECTORY: &str = "state-displaced";
const SENTINEL_FILE: &str = "replacement-sentinel";
const SENTINEL_BYTES: &[u8] = b"preserve-replacement";
const NAMESPACE_EXIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
const NO_CONNECTION_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(50);
const OBSERVED_REVISION: &str = "observed-revision";

fn paths() -> (tempfile::TempDir, BrokerPaths) {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join("state")).unwrap();
    (root, paths)
}

fn replace_state(paths: &BrokerPaths) -> (PathBuf, BrokerPaths) {
    let displaced = paths
        .state_dir()
        .parent()
        .unwrap()
        .join(DISPLACED_DIRECTORY);
    std::fs::rename(paths.state_dir(), &displaced).unwrap();
    let replacement = BrokerPaths::new(paths.state_dir()).unwrap();
    (displaced, replacement)
}

fn write_sentinel(paths: &BrokerPaths) -> PathBuf {
    let sentinel = paths.state_dir().join(SENTINEL_FILE);
    std::fs::write(&sentinel, SENTINEL_BYTES).unwrap();
    sentinel
}

#[tokio::test]
async fn external_reconcile_lock_survives_state_replacement() {
    let (_root, paths) = paths();
    let guard =
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT)
            .await
            .unwrap();
    let (_displaced, replacement) = replace_state(&paths);
    assert_eq!(paths.reconcile_lock(), replacement.reconcile_lock());
    assert_eq!(
        paths.launch_publication_lock(),
        replacement.launch_publication_lock()
    );
    assert!(
        BrokerReconcileGuard::acquire(&replacement, tokio::time::Instant::now())
            .await
            .is_err()
    );
    assert!(guard.verify().is_err());
    assert!(guard.verify().is_err());
    drop(guard);
    BrokerReconcileGuard::acquire(
        &replacement,
        tokio::time::Instant::now() + BROKER_READY_TIMEOUT,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn instance_owner_blocks_replacement_and_preserves_visible_state() {
    let (_root, paths) = paths();
    let sockets = BrokerSockets::bind(&paths).await.unwrap();
    let (_displaced, replacement) = replace_state(&paths);
    let sentinel = write_sentinel(&replacement);
    assert_eq!(paths.instance_lock(), replacement.instance_lock());
    assert!(BrokerSockets::bind(&replacement).await.is_err());
    assert!(sockets.verify_namespace().is_err());
    assert!(sockets.verify_namespace().is_err());
    drop(sockets);
    assert_eq!(std::fs::read(&sentinel).unwrap(), SENTINEL_BYTES);
    drop(BrokerSockets::bind(&replacement).await.unwrap());
}

#[tokio::test]
async fn bind_detects_replacement_before_preparing_endpoints() {
    let (_root, paths) = paths();
    let state = paths.state_dir().to_owned();
    let displaced = state.parent().unwrap().join(DISPLACED_DIRECTORY);
    let error = BrokerSockets::bind_with_namespace_hook(&paths, || {
        std::fs::rename(&state, &displaced)?;
        BrokerPaths::new(&state)?;
        Ok(())
    })
    .await
    .err()
    .unwrap();
    assert!(error.to_string().contains(BROKER_NAMESPACE_LOST_CONTEXT));
    assert!(!paths.runtime_socket().exists());
    assert!(!paths.management_socket().exists());
}

#[tokio::test]
async fn stale_paths_cannot_capture_a_replacement_as_their_namespace() {
    let (_root, paths) = paths();
    let (displaced, replacement) = replace_state(&paths);
    let sentinel = write_sentinel(&replacement);

    assert!(
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now())
            .await
            .is_err()
    );
    assert!(BrokerSockets::bind(&paths).await.is_err());
    assert!(!displaced.join(RUNTIME_SOCKET_FILE).exists());
    assert!(!displaced.join(MANAGEMENT_SOCKET_FILE).exists());
    assert_eq!(fs::read(sentinel).unwrap(), SENTINEL_BYTES);
}

#[tokio::test]
async fn partial_bind_is_compensated_in_the_retained_namespace() {
    let (_root, paths) = paths();
    let mut displaced = None;
    let mut sentinel = None;
    let error = BrokerSockets::bind_with_socket_hook(&paths, |stage| {
        if stage == BrokerSocketBindStage::RuntimePublished {
            let (old_state, replacement) = replace_state(&paths);
            sentinel = Some(write_sentinel(&replacement));
            displaced = Some(old_state);
        }
        Ok(())
    })
    .await
    .err()
    .unwrap();

    assert!(error.to_string().contains(BROKER_NAMESPACE_LOST_CONTEXT));
    let displaced = displaced.unwrap();
    assert!(!displaced.join(RUNTIME_SOCKET_FILE).exists());
    assert!(!displaced.join(MANAGEMENT_SOCKET_FILE).exists());
    assert!(!paths.runtime_socket().exists());
    assert!(!paths.management_socket().exists());
    assert_eq!(fs::read(sentinel.unwrap()).unwrap(), SENTINEL_BYTES);
}

#[tokio::test]
async fn server_exits_after_namespace_loss_without_touching_replacement() {
    let mut harness = Harness::start(FakeFactory::empty(Vec::new())).await;
    let paths = harness.paths.clone();
    let (_displaced, replacement) = replace_state(&paths);
    let sentinel = write_sentinel(&replacement);
    assert!(BrokerSockets::bind(&replacement).await.is_err());
    let error = tokio::time::timeout(NAMESPACE_EXIT_TIMEOUT, harness.wait_namespace_lost())
        .await
        .unwrap();
    assert!(error.to_string().contains(BROKER_NAMESPACE_LOST_CONTEXT));
    assert_eq!(std::fs::read(&sentinel).unwrap(), SENTINEL_BYTES);
    drop(BrokerSockets::bind(&replacement).await.unwrap());
}

#[tokio::test]
async fn stale_reconcile_never_sends_shutdown_to_replacement_socket() {
    let (_root, paths) = paths();
    let guard =
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT)
            .await
            .unwrap();
    let sockets = BrokerSockets::bind(&paths).await.unwrap();
    let retirement = BrokerRetirement::capture(&guard).unwrap();
    let (_displaced, replacement) = replace_state(&paths);
    let listener = UnixListener::bind(replacement.management_socket()).unwrap();
    std::fs::set_permissions(
        replacement.management_socket(),
        std::fs::Permissions::from_mode(super::super::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    let client = BrokerClient::new(replacement.management_socket());
    assert!(
        retirement
            .transition(
                &client,
                OBSERVED_REVISION,
                &guard,
                tokio::time::Instant::now(),
            )
            .await
            .is_err()
    );
    assert!(
        tokio::time::timeout(NO_CONNECTION_TIMEOUT, listener.accept())
            .await
            .is_err()
    );
    drop(sockets);
}
