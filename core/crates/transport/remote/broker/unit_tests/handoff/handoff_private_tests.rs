use super::*;
use crate::remote_broker::BROKER_READY_TIMEOUT;
use crate::remote_broker::handoff_endpoint::{finish_retirement_probe_with_hook, socket_name};
use crate::remote_broker::socket_directory::SocketDirectory;
use crate::remote_broker::socket_identity::socket_identity;
use crate::remote_broker::socket_probe::BrokerSocketProbe;
use crate::remote_broker::sockets::BrokerSockets;
use std::cell::RefCell;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tokio::net::UnixListener;

const LOCK_TIMEOUT_MESSAGE: &str = "timed out waiting";
const SOCKET_BYTES: &[u8] = b"not-a-socket";
const DISPLACED_STATE_DIRECTORY: &str = "retirement-displaced-state";
const SENTINEL_FILE: &str = "replacement-sentinel";
const SENTINEL_BYTES: &[u8] = b"preserve-replacement";

fn paths() -> (tempfile::TempDir, BrokerPaths) {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join("state")).unwrap();
    (root, paths)
}

fn bind(path: &Path) -> UnixListener {
    let listener = UnixListener::bind(path).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    listener
}

async fn reconcile_guard(paths: &BrokerPaths) -> BrokerReconcileGuard {
    BrokerReconcileGuard::acquire(paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT)
        .await
        .unwrap()
}

#[tokio::test]
async fn reconcile_lock_serializes_callers_and_releases_on_drop() {
    let (_root, paths) = paths();
    let deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    let first = BrokerReconcileGuard::acquire(&paths, deadline)
        .await
        .unwrap();
    let error = BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now())
        .await
        .err()
        .unwrap();
    assert!(error.to_string().contains(LOCK_TIMEOUT_MESSAGE));
    drop(first);
    BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT)
        .await
        .unwrap();
}

#[tokio::test]
async fn instance_lock_prevents_retirement_until_owner_exits() {
    let (_root, paths) = paths();
    let guard = reconcile_guard(&paths).await;
    let sockets = BrokerSockets::bind(&paths).await.unwrap();
    let retirement = BrokerRetirement::capture(&guard).unwrap();
    assert!(!broker_instance_available(&paths).unwrap());
    assert!(!retirement.retire_if_unlocked(&paths, &guard).await.unwrap());
    drop(sockets);
    assert!(retirement.retire_if_unlocked(&paths, &guard).await.unwrap());
    assert!(broker_instance_available(&paths).unwrap());
}

#[tokio::test]
async fn captured_stale_endpoints_are_removed_only_when_unchanged() {
    let (_root, paths) = paths();
    let guard = reconcile_guard(&paths).await;
    let runtime = bind(&paths.runtime_socket());
    let management = bind(&paths.management_socket());
    let retirement = BrokerRetirement::capture(&guard).unwrap();
    drop(runtime);
    drop(management);
    assert!(retirement.retire_if_unlocked(&paths, &guard).await.unwrap());
    assert!(!paths.runtime_socket().exists());
    assert!(!paths.management_socket().exists());

    let old = bind(&paths.runtime_socket());
    let replacement_guard = BrokerRetirement::capture(&guard).unwrap();
    drop(old);
    std::fs::remove_file(paths.runtime_socket()).unwrap();
    let replacement = bind(&paths.runtime_socket());
    assert!(
        replacement_guard
            .retire_if_unlocked(&paths, &guard)
            .await
            .unwrap()
    );
    assert!(paths.runtime_socket().exists());
    drop(replacement);
}

#[tokio::test]
async fn active_missing_and_unsafe_endpoints_have_distinct_results() {
    let (_root, paths) = paths();
    let guard = reconcile_guard(&paths).await;
    let active = bind(&paths.runtime_socket());
    let active_guard = BrokerRetirement::capture(&guard).unwrap();
    assert!(
        !active_guard
            .retire_if_unlocked(&paths, &guard)
            .await
            .unwrap()
    );
    drop(active);
    std::fs::remove_file(paths.runtime_socket()).unwrap();
    assert!(
        active_guard
            .retire_if_unlocked(&paths, &guard)
            .await
            .unwrap()
    );

    let stale = bind(&paths.runtime_socket());
    let unsafe_guard = BrokerRetirement::capture(&guard).unwrap();
    drop(stale);
    std::fs::remove_file(paths.runtime_socket()).unwrap();
    std::fs::write(paths.runtime_socket(), SOCKET_BYTES).unwrap();
    assert!(
        unsafe_guard
            .retire_if_unlocked(&paths, &guard)
            .await
            .is_err()
    );
    assert!(BrokerRetirement::capture(&guard).is_err());
}

#[tokio::test]
async fn namespace_replacement_between_probe_and_remove_is_preserved() {
    let (root, paths) = paths();
    let stale = bind(&paths.runtime_socket());
    let expected = socket_identity(&paths.runtime_socket()).unwrap();
    let directory = SocketDirectory::new(paths.state_directory().unwrap());
    let name = socket_name(RUNTIME_SOCKET_FILE).unwrap();
    drop(stale);
    let state = paths.state_dir().to_owned();
    let displaced = root.path().join(DISPLACED_STATE_DIRECTORY);
    let replacement_listener = RefCell::new(None);
    let sentinel = RefCell::new(None);

    let removed = finish_retirement_probe_with_hook(
        &directory,
        &name,
        expected,
        BrokerSocketProbe::Refused,
        || {
            std::fs::rename(&state, &displaced)?;
            let replacement = BrokerPaths::new(&state)?;
            let sentinel_path = replacement.state_dir().join(SENTINEL_FILE);
            std::fs::write(&sentinel_path, SENTINEL_BYTES)?;
            sentinel.replace(Some(sentinel_path));
            replacement_listener.replace(Some(bind(&replacement.runtime_socket())));
            Ok(())
        },
    )
    .unwrap();

    assert!(removed);
    assert!(!displaced.join(RUNTIME_SOCKET_FILE).exists());
    assert!(paths.runtime_socket().exists());
    assert_eq!(
        std::fs::read(sentinel.borrow().as_ref().unwrap()).unwrap(),
        SENTINEL_BYTES
    );
    drop(replacement_listener);
}
