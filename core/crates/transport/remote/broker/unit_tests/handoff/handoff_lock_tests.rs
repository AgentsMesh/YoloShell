use super::*;
use crate::remote_broker::BROKER_READY_TIMEOUT;
use crate::remote_broker::handoff_endpoint::{finish_retirement_probe, socket_name};
use crate::remote_broker::socket_directory::SocketDirectory;
use crate::remote_broker::socket_identity::socket_identity;
use crate::remote_broker::socket_probe::BrokerSocketProbe;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tokio::net::UnixListener;

const PUBLIC_FILE_MODE: u32 = 0o644;
const LOCK_TARGET_FILE: &str = "lock-target";
const LOCK_TARGET_BYTES: &[u8] = b"preserve-coordination-target";
const LOCK_BYTES: &[u8] = b"lock";
const LOCK_PATH_CHANGED_MESSAGE: &str = "verify locked private file path";

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

#[tokio::test]
async fn coordination_lock_rejects_unsafe_types_and_permissions() {
    let (_root, paths) = paths();
    std::fs::write(paths.reconcile_lock(), LOCK_BYTES).unwrap();
    std::fs::set_permissions(
        paths.reconcile_lock(),
        std::fs::Permissions::from_mode(PUBLIC_FILE_MODE),
    )
    .unwrap();
    assert!(
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT,)
            .await
            .is_err()
    );
    std::fs::remove_file(paths.reconcile_lock()).unwrap();
    std::fs::create_dir(paths.reconcile_lock()).unwrap();
    assert!(broker_instance_available(&paths).is_ok());
    assert!(
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT,)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn coordination_locks_reject_hardlinks_without_mutating_targets() {
    let (_root, paths) = paths();
    let target = paths.state_dir().join(LOCK_TARGET_FILE);
    std::fs::write(&target, LOCK_TARGET_BYTES).unwrap();
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    std::fs::hard_link(&target, paths.reconcile_lock()).unwrap();
    assert!(
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT,)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&target).unwrap(), LOCK_TARGET_BYTES);
    std::fs::remove_file(paths.reconcile_lock()).unwrap();
    std::fs::hard_link(&target, paths.instance_lock()).unwrap();
    assert!(broker_instance_available(&paths).is_err());
    assert_eq!(std::fs::read(target).unwrap(), LOCK_TARGET_BYTES);
}

#[tokio::test]
async fn reconcile_lock_rejects_path_replacement_after_flock() {
    let (_root, paths) = paths();
    let lock_path = paths.reconcile_lock().to_owned();
    let error = acquire_reconcile_with_hook(&paths, || {
        std::fs::remove_file(&lock_path)?;
        std::fs::write(&lock_path, [])?;
        std::fs::set_permissions(
            &lock_path,
            std::fs::Permissions::from_mode(PRIVATE_FILE_MODE),
        )?;
        Ok(())
    })
    .await
    .err()
    .unwrap();
    assert!(error.to_string().contains(LOCK_PATH_CHANGED_MESSAGE));
    BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT)
        .await
        .unwrap();
}

#[tokio::test]
async fn reconcile_hook_success_returns_a_verified_guard() {
    let (_root, paths) = paths();

    let guard = acquire_reconcile_with_hook(&paths, || Ok(()))
        .await
        .unwrap();

    guard.verify().unwrap();
}

#[tokio::test]
async fn retirement_probe_timeout_and_other_errors_preserve_the_socket() {
    let (_root, paths) = paths();
    let listener = bind(&paths.runtime_socket());
    let identity = socket_identity(&paths.runtime_socket()).unwrap();
    let directory = SocketDirectory::new(paths.state_directory().unwrap());
    let name = socket_name(RUNTIME_SOCKET_FILE).unwrap();
    let timeout = finish_retirement_probe(&directory, &name, identity, BrokerSocketProbe::TimedOut)
        .err()
        .unwrap();
    assert!(
        timeout
            .to_string()
            .contains(BROKER_SOCKET_PROBE_TIMEOUT_CONTEXT)
    );
    let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    assert!(
        finish_retirement_probe(
            &directory,
            &name,
            identity,
            BrokerSocketProbe::Indeterminate(denied),
        )
        .is_err()
    );
    assert_eq!(socket_identity(&paths.runtime_socket()).unwrap(), identity);
    drop(listener);
}

#[tokio::test(start_paused = true)]
async fn reconciliation_does_not_admit_a_lock_at_its_exact_deadline() {
    let (_root, paths) = paths();
    let held =
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT)
            .await
            .unwrap();
    let waiting_paths = paths.clone();
    let deadline = tokio::time::Instant::now() + BROKER_READY_POLL_INTERVAL;
    let waiting =
        tokio::spawn(async move { BrokerReconcileGuard::acquire(&waiting_paths, deadline).await });
    tokio::task::yield_now().await;

    drop(held);
    tokio::time::advance(BROKER_READY_POLL_INTERVAL).await;

    assert!(waiting.await.unwrap().is_err());
}
