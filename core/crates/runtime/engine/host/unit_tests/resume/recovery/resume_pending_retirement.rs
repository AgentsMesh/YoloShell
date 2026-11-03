use super::*;

const INVALID_EXIT_BYTES: &[u8] = b"invalid-exit-record";

#[tokio::test(start_paused = true)]
async fn pending_retirement_timeout_does_not_disable_verified_crash_reconciliation() {
    let fixture = AutomaticCrashFixture::new();
    let sequence = fixture.append_output();
    fixture.settle_custody();
    let mut manifest = fixture.manifest.clone();
    let selected = manifest_snapshot(&manifest);
    let session_guard = fixture.host.session_locks.lock(manifest.terminal_id).await;

    let error = fixture
        .host
        .settle_disconnected_previous_runtime(&mut manifest, &fixture.runtime)
        .await
        .unwrap_err()
        .into_explicit_error();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
    assert!(error.retryable);
    assert_eq!(manifest.recovery_state, fixture.manifest.recovery_state);
    assert_manifest_snapshot(&manifest, &selected);
    assert_eq!(
        fixture.host.automatic_restore_decision(&manifest),
        AutomaticRestore::Eligible
    );
    drop(session_guard);

    let result = fixture.host.reconcile().await;
    assert_eq!(result.needs_recreate, RECONCILIATION_FAILURE_COUNT);
    let durable: TerminalManifest =
        read_json(&fixture.host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::NeedsRecreate);
    let exit: ExitRecord = read_json(&fixture.host.paths.exit_record(manifest.terminal_id)).unwrap();
    assert_eq!(exit.reason, ExitReason::WorkerCrash);
    assert_eq!(exit.runtime_epoch, fixture.runtime.runtime_epoch);
    assert_eq!(exit.last_output_sequence, sequence);
    let current: RuntimeRecord =
        read_json(&fixture.host.paths.runtime_record(manifest.terminal_id)).unwrap();
    assert_eq!(current, fixture.runtime);
}

#[tokio::test(start_paused = true)]
async fn pending_retirement_ack_timeout_preserves_the_existing_recovery_state() {
    let fixture = AutomaticCrashFixture::new();
    fixture.host.secondary_custody.install_test_retired_lease(
        &fixture.host,
        &fixture.runtime,
        secondary_custody::TestRetiredCustodyState::ReleaseAckPending,
    );
    let mut manifest = fixture.manifest.clone();
    let snapshot = manifest_snapshot(&manifest);
    let error = fixture
        .host
        .settle_disconnected_previous_runtime(&mut manifest, &fixture.runtime)
        .await
        .unwrap_err()
        .into_explicit_error();
    assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
    assert_eq!(manifest.recovery_state, fixture.manifest.recovery_state);
    assert_manifest_snapshot(&manifest, &snapshot);
    let durable: TerminalManifest =
        read_json(&fixture.host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_manifest_snapshot(&durable, &snapshot);
    assert_eq!(
        fixture.host.automatic_restore_decision(&manifest),
        AutomaticRestore::AwaitingRuntimeSettlement
    );
    assert!(!fixture.host.paths.exit_record(manifest.terminal_id).exists());
}

#[derive(Clone, Copy)]
enum InvalidRetirement {
    MissingCustody,
    UncertainRelease,
    ReplacedEpoch,
    InvalidExit,
}

#[tokio::test(start_paused = true)]
async fn pending_retirement_failure_still_isolates_changed_or_unproven_evidence() {
    for fault in [
        InvalidRetirement::MissingCustody,
        InvalidRetirement::UncertainRelease,
        InvalidRetirement::ReplacedEpoch,
        InvalidRetirement::InvalidExit,
    ] {
        let fixture = AutomaticCrashFixture::new();
        match fault {
            InvalidRetirement::MissingCustody => {}
            InvalidRetirement::UncertainRelease => {
                fixture.host.secondary_custody.install_test_retired_lease(
                    &fixture.host,
                    &fixture.runtime,
                    secondary_custody::TestRetiredCustodyState::ReleaseUncertain,
                );
            }
            InvalidRetirement::ReplacedEpoch => {
                fixture.settle_custody();
                let mut replacement = fixture.runtime.clone();
                replacement.runtime_epoch = Uuid::new_v4();
                atomic_write_json(
                    &fixture.host.paths.runtime_record(replacement.terminal_id),
                    &replacement,
                )
                .unwrap();
            }
            InvalidRetirement::InvalidExit => {
                fixture.settle_custody();
                write_private_bytes(
                    &fixture.host.paths.exit_record(fixture.manifest.terminal_id),
                    INVALID_EXIT_BYTES,
                );
            }
        }
        let mut manifest = fixture.manifest.clone();
        let error = fixture
            .host
            .settle_disconnected_previous_runtime(&mut manifest, &fixture.runtime)
            .await
            .unwrap_err()
            .into_explicit_error();
        assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
        assert_eq!(manifest.recovery_state, RecoveryState::Uncertain);
        let durable: TerminalManifest =
            read_json(&fixture.host.paths.manifest(manifest.terminal_id)).unwrap();
        assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
    }
}

#[tokio::test(start_paused = true)]
async fn pending_retirement_does_not_reopen_an_explicitly_terminated_runtime() {
    let fixture = AutomaticCrashFixture::new();
    let tombstone = crate::model::TerminationTombstone::user_request(
        fixture.manifest.terminal_id,
        fixture.runtime.runtime_epoch,
        true,
    );
    atomic_write_json(
        &fixture
            .host
            .paths
            .terminate_tombstone(fixture.manifest.terminal_id),
        &tombstone,
    )
    .unwrap();
    let mut manifest = fixture.manifest.clone();
    fixture
        .host
        .settle_disconnected_previous_runtime(&mut manifest, &fixture.runtime)
        .await
        .unwrap();
    assert_eq!(
        fixture.host.automatic_restore_decision(&manifest),
        AutomaticRestore::Ineligible
    );
    assert!(!fixture.host.paths.exit_record(manifest.terminal_id).exists());
    let current: RuntimeRecord =
        read_json(&fixture.host.paths.runtime_record(manifest.terminal_id)).unwrap();
    assert_eq!(current, fixture.runtime);
}
