use crate::host::tests::*;
use super::*;

const RECONCILE_HOME_NAME: &str = "reconcile-home";
const DISPLACED_RECONCILE_SESSIONS: &str = "displaced-reconcile-sessions";
const RECOVERABLE_ERROR_DETAIL: &str = "runtime evidence failure";
const EXPECTED_SINGLE_RECONCILE_FAILURE: usize = 1;
const PANICKING_RECONCILE_SESSION_NAME: &str = "panicking-reconcile-session";

#[tokio::test]
async fn reconcile_reports_namespace_loss_without_touching_displaced_state() {
    let root = tempdir().unwrap();
    let home = root.path().join(RECONCILE_HOME_NAME);
    let host = Host::open(HostConfig::new(&home)).unwrap();
    let manifest = fixture_manifest(&home);
    seed_recovery_manifest(&host, &manifest).await;
    let sessions = host.paths.sessions_dir();
    let displaced = root.path().join(DISPLACED_RECONCILE_SESSIONS);
    std::fs::rename(&sessions, &displaced).unwrap();
    crate::paths::directory::PrivateDirectory::ensure(&sessions).unwrap();

    let result = host.reconcile().await;

    assert_eq!(result.unreachable, EXPECTED_SINGLE_RECONCILE_FAILURE);
    assert_eq!(result.reattached, usize::default());
    assert_eq!(result.exited, usize::default());
    assert_eq!(result.needs_recreate, usize::default());
    assert!(host.namespace.verify().is_err());
    assert!(displaced.join(manifest.terminal_id.to_string()).is_dir());
}

#[tokio::test]
async fn reconcile_isolates_poisoned_creation_state_before_runtime_recovery() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    lock_recover(&host.creation_capabilities).poisoned = true;

    let result = host.reconcile_manifest(manifest.clone()).await;

    assert_eq!(result.unreachable, EXPECTED_SINGLE_RECONCILE_FAILURE);
    assert_eq!(result.needs_recreate, usize::default());
    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Created);
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
}

#[tokio::test]
async fn reconcile_isolates_a_catalog_row_that_fails_semantic_decoding() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    corrupt_catalog_manifest(&host, manifest.terminal_id);

    let result = host.reconcile_manifest(manifest.clone()).await;

    assert_eq!(result.unreachable, EXPECTED_SINGLE_RECONCILE_FAILURE);
    assert_eq!(result.needs_recreate, usize::default());
    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_eq!(durable.terminal_id, manifest.terminal_id);
    assert_eq!(durable.recovery_state, RecoveryState::Created);
}

#[tokio::test]
async fn reconcile_marks_a_dead_pending_bootstrap_uncertain_without_respawn() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut record = dead_runtime(&host, &manifest);
    record.bootstrap_delivery = BootstrapDelivery::Pending;
    persist_host_session(&host, &manifest, &record);

    let result = host.reconcile_manifest(manifest.clone()).await;

    assert_eq!(result.unreachable, EXPECTED_SINGLE_RECONCILE_FAILURE);
    assert_eq!(result.needs_recreate, usize::default());
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    let catalog = host
        .catalog_get(manifest.terminal_id)
        .await
        .unwrap()
        .unwrap();
    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_eq!(catalog.recovery_state, RecoveryState::Uncertain);
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
}

#[tokio::test]
async fn reconcile_contains_a_panicking_session_sweep() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.name = PANICKING_RECONCILE_SESSION_NAME.into();
    seed_recovery_manifest(&host, &manifest).await;
    *lock_recover(&host.catalog_test_probes.reconcile_panic) = Some(manifest.terminal_id);

    let result = host.reconcile().await;

    assert_eq!(result.unreachable, EXPECTED_SINGLE_RECONCILE_FAILURE);
    assert_eq!(result.reattached, usize::default());
    assert_eq!(result.exited, usize::default());
    assert_eq!(result.needs_recreate, usize::default());
    let retained = host
        .catalog_get(manifest.terminal_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retained.terminal_id, manifest.terminal_id);
    assert_eq!(retained.name, manifest.name);
    assert_eq!(retained.recovery_state, manifest.recovery_state);
    assert_eq!(retained.archived, manifest.archived);
}

#[test]
fn runtime_loss_recovery_code_classification_is_exhaustive() {
    for code in [
        crate::protocol::error_code::HOST_REBOOTED,
        crate::protocol::error_code::WORKER_UNREACHABLE,
        crate::protocol::error_code::IDENTITY_MISMATCH,
    ] {
        let error = ErrorBody::new(code, RECOVERABLE_ERROR_DETAIL, true);
        assert!(recoverable_runtime_loss(&error), "rejected {code}");
    }
    for code in [
        crate::protocol::error_code::INTERNAL,
        crate::protocol::error_code::CONFLICT,
        crate::protocol::error_code::NOT_FOUND,
    ] {
        let error = ErrorBody::new(code, RECOVERABLE_ERROR_DETAIL, false);
        assert!(!recoverable_runtime_loss(&error), "accepted {code}");
    }
}

#[path = "reconcile_containment_tests.rs"]
mod reconcile_containment_tests;
