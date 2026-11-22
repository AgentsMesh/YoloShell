use crate::host::tests::*;
use super::*;
#[cfg(target_os = "macos")]
const SETTLEMENT_TEST_DEAD_WORKER_PID: u32 = u32::MAX;
#[cfg(target_os = "macos")]
const SETTLEMENT_TEST_DEAD_WORKER_START: u64 = 1;
#[cfg(target_os = "macos")]
const SETTLEMENT_TEST_JOURNAL_OUTPUT: &[u8] = b"settled-worker-crash-output";
#[cfg(target_os = "macos")]
const SETTLEMENT_TEST_SOCKET: &str = "settlement-worker.sock";
#[cfg(target_os = "macos")]
const SETTLEMENT_TEST_HOST_BOOT: &str = "settlement-host-boot";
#[cfg(target_os = "macos")]
const SETTLEMENT_TEST_UNKNOWN_BOOT_SUFFIX: &str = "settlement-runtime";
#[cfg(target_os = "macos")]
const SETTLEMENT_TEST_RECONCILE_ATTEMPTS: usize = 2;

#[cfg(target_os = "macos")]
#[tokio::test]
async fn worker_crash_waits_for_child_without_poisoning_automatic_recovery() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut child = spawn_resume_test_child();
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(SETTLEMENT_TEST_SOCKET),
    );
    let worker = reaped_test_process();
    runtime.worker_pid = worker.pid;
    runtime.worker_start_time = worker.start_time;
    runtime.child_pid = child.id();
    runtime.child_start_time = process::process_start_time(child.id()).unwrap();
    persist_host_session(&host, &manifest, &runtime);
    let custody = host
        .secondary_custody
        .install_test_active_lease(&host, &runtime);

    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::AwaitingRuntimeSettlement
    );
    let waiting = host.reconcile().await;
    assert_eq!(waiting.unreachable, 1);
    assert_eq!(waiting.needs_recreate, 0);
    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Created);

    stop_resume_test_child(&mut child);
    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::AwaitingRuntimeSettlement
    );
    host.secondary_custody
        .disconnect_test_active_lease(&custody);
    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::AwaitingRuntimeSettlement
    );
    let mut journal = crate::journal::Journal::open(
        host.paths.journal(manifest.terminal_id),
        runtime.runtime_epoch,
    )
    .unwrap();
    let sequence = journal
        .append(SETTLEMENT_TEST_JOURNAL_OUTPUT)
        .unwrap()
        .sequence;
    drop(journal);
    host.secondary_custody
        .retire_test_disconnected_lease(&host, &runtime, &custody);
    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::Eligible
    );

    host.ensure_automatic_worker_crash_exit_inner(&manifest)
        .await
        .unwrap();
    let exit: ExitRecord = read_json(&host.paths.exit_record(manifest.terminal_id)).unwrap();
    assert_eq!(exit.reason, ExitReason::WorkerCrash);
    assert_eq!(exit.last_output_sequence, sequence);
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn retired_custody_with_a_live_child_awaits_process_exit_convergence() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(SETTLEMENT_TEST_SOCKET),
    );
    runtime.worker_pid = SETTLEMENT_TEST_DEAD_WORKER_PID;
    runtime.worker_start_time = SETTLEMENT_TEST_DEAD_WORKER_START;
    persist_host_session(&host, &manifest, &runtime);
    host.secondary_custody.install_test_retired_lease(
        &host,
        &runtime,
        secondary_custody::TestRetiredCustodyState::ReleaseAckPending,
    );

    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::AwaitingRuntimeSettlement
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn absent_custody_with_a_live_child_converges_once_to_uncertain() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(SETTLEMENT_TEST_SOCKET),
    );
    runtime.worker_pid = SETTLEMENT_TEST_DEAD_WORKER_PID;
    runtime.worker_start_time = SETTLEMENT_TEST_DEAD_WORKER_START;
    persist_host_session(&host, &manifest, &runtime);

    for _attempt in 0..SETTLEMENT_TEST_RECONCILE_ATTEMPTS {
        let result = host.reconcile().await;
        assert_eq!(result.unreachable, RECONCILIATION_FAILURE_COUNT);
        assert_eq!(result.needs_recreate, usize::default());
    }
    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn unknown_boot_runtime_loss_converges_to_uncertain() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = SETTLEMENT_TEST_HOST_BOOT.into();
    let manifest = fixture_manifest(directory.path());
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(SETTLEMENT_TEST_SOCKET),
    );
    runtime.boot_id = format!(
        "{}{}",
        crate::process::unknown_boot_id_for_test(),
        SETTLEMENT_TEST_UNKNOWN_BOOT_SUFFIX
    );
    runtime.worker_pid = SETTLEMENT_TEST_DEAD_WORKER_PID;
    runtime.worker_start_time = SETTLEMENT_TEST_DEAD_WORKER_START;
    runtime.child_pid = SETTLEMENT_TEST_DEAD_WORKER_PID;
    runtime.child_start_time = SETTLEMENT_TEST_DEAD_WORKER_START;
    persist_host_session(&host, &manifest, &runtime);

    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::BlockedByRuntime
    );
    let result = host.reconcile().await;
    assert_eq!(result.unreachable, RECONCILIATION_FAILURE_COUNT);
    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn missing_crash_journal_converges_without_repeated_automatic_attempts() {
    let fixture = AutomaticCrashFixture::new();
    fixture.settle_custody();
    fixture.remove_journal_evidence();

    for _attempt in 0..SETTLEMENT_TEST_RECONCILE_ATTEMPTS {
        let result = fixture.host.reconcile().await;
        assert_eq!(result.unreachable, RECONCILIATION_FAILURE_COUNT);
        assert_eq!(result.needs_recreate, usize::default());
    }
    let durable: TerminalManifest =
        read_json(&fixture.host.paths.manifest(fixture.manifest.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
    assert!(
        !fixture
            .host
            .paths
            .journal(fixture.manifest.terminal_id)
            .exists()
    );
    assert!(
        !fixture
            .host
            .paths
            .exit_record(fixture.manifest.terminal_id)
            .exists()
    );
}
