use crate::host::tests::*;
use super::*;

#[cfg(target_os = "macos")]
const UNCERTAIN_CUSTODY_SOCKET: &str = "uncertain-custody.sock";

#[cfg(target_os = "macos")]
#[tokio::test]
async fn release_uncertain_custody_converges_manifest_instead_of_waiting_forever() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(UNCERTAIN_CUSTODY_SOCKET),
    );
    install_reaped_runtime_processes(&mut runtime);
    persist_host_session(&host, &manifest, &runtime);
    host.secondary_custody.install_test_retired_lease(
        &host,
        &runtime,
        secondary_custody::TestRetiredCustodyState::ReleaseUncertain,
    );

    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::BlockedByRuntime
    );
    let result = host.reconcile().await;
    assert_eq!(result.unreachable, RECONCILIATION_FAILURE_COUNT);
    assert_eq!(result.needs_recreate, usize::default());
    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
}
