use crate::host::tests::*;
use super::*;

#[cfg(target_os = "macos")]
const ACTIVE_CUSTODY_RACE_SOCKET: &str = "active-custody-race.sock";
#[cfg(target_os = "macos")]
const ACTIVE_CUSTODY_RACE_ATTEMPTS: usize = 2;
#[cfg(target_os = "macos")]
const ACTIVE_CUSTODY_PREVIOUS_BOOT: &str = "active-custody-previous-boot";

#[cfg(target_os = "macos")]
#[tokio::test]
async fn active_custody_with_current_worker_and_unreachable_socket_does_not_poison_recovery() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(ACTIVE_CUSTODY_RACE_SOCKET),
    );
    persist_host_session(&host, &manifest, &runtime);
    host.secondary_custody
        .install_test_active_lease(&host, &runtime);

    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::AwaitingRuntimeSettlement
    );
    for _attempt in 0..ACTIVE_CUSTODY_RACE_ATTEMPTS {
        let result = host.reconcile().await;
        assert_eq!(result.unreachable, RECONCILIATION_FAILURE_COUNT);
        assert_eq!(result.needs_recreate, usize::default());
    }

    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Created);
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn reboot_observed_after_active_custody_decision_keeps_waiting_for_settlement() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let manifest = fixture_manifest(directory.path());
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(ACTIVE_CUSTODY_RACE_SOCKET),
    );
    persist_host_session(&host, &manifest, &runtime);
    host.secondary_custody
        .install_test_active_lease(&host, &runtime);
    let (reached, observed) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    *lock_recover(&host.catalog_test_probes.restore_decision) = Some(ReconcileRestoreProbe {
        reached,
        release: released,
    });
    let task_host = Arc::clone(&host);
    let task = tokio::spawn(async move { task_host.reconcile_manifest(manifest).await });
    assert_eq!(
        observed.await.unwrap(),
        AutomaticRestore::AwaitingRuntimeSettlement
    );
    runtime.boot_id = ACTIVE_CUSTODY_PREVIOUS_BOOT.into();
    atomic_write_json(&host.paths.runtime_record(runtime.terminal_id), &runtime).unwrap();
    release.send(()).unwrap();

    let result = task.await.unwrap();
    assert_eq!(result.unreachable, RECONCILIATION_FAILURE_COUNT);
    assert_eq!(result.needs_recreate, usize::default());
    let durable: TerminalManifest = read_json(&host.paths.manifest(runtime.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Created);
    assert!(!host.paths.worker_launch(runtime.terminal_id).exists());
}
