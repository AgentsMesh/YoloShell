use crate::host::tests::*;
use super::*;

fn failed_resume_checkpoint(
    transaction: &ResumeTransaction,
    runtime: &RuntimeRecord,
) -> crate::worker::StartupCheckpointEvidence {
    let fence = crate::worker::worker_startup_launch_fence_for_test(&transaction.launch).unwrap();
    crate::worker::StartupCheckpointEvidence {
        schema_version: crate::worker::TEST_WORKER_STARTUP_CHECKPOINT_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        controller_generation: transaction.launch.controller_generation,
        launch_sha256: fence.launch_sha256,
        predecessor_runtime_sha256: None,
        pending_pty_custody_rollover: None,
        boot_id: runtime.boot_id.clone(),
        worker: crate::worker::StartupProcessIdentity {
            pid: runtime.worker_pid,
            start_time: runtime.worker_start_time,
        },
        child: Some(crate::worker::StartupProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        }),
        stage: crate::worker::StartupStage::CommandExec,
        phase: crate::worker::StartupPhase::Entered,
        status: crate::worker::StartupStatus::Active,
        successor: None,
    }
}

fn persist_failed_resume_checkpoint(
    host: &Host,
    transaction: &ResumeTransaction,
    runtime: &RuntimeRecord,
) -> crate::worker::StartupCheckpointEvidence {
    let checkpoint = failed_resume_checkpoint(transaction, runtime);
    host.write_namespace_json(
        &host.paths.worker_startup_checkpoint(runtime.terminal_id),
        &checkpoint,
    )
    .unwrap();
    checkpoint
}

#[tokio::test]
async fn dead_failed_resume_rearms_and_replays_one_successor() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let (manifest, transaction, mut runtime) = dead_survivor_rearm_fixture(&host, directory.path());
    runtime.bootstrap_delivery = BootstrapDelivery::Failed;
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &runtime)
        .unwrap();
    persist_failed_resume_checkpoint(&host, &transaction, &runtime);

    let first = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await
        .unwrap_err()
        .into_explicit_error();
    assert_eq!(first.code, crate::protocol::error_code::WORKER_START_FAILED);
    assert_eq!(first.message, WORKER_START_PUBLIC_DETAIL);
    let replacement: ResumeTransaction =
        read_json(&host.paths.resume_transaction(manifest.terminal_id)).unwrap();
    assert_ne!(
        replacement.transaction_digest,
        transaction.transaction_digest
    );
    assert_eq!(replacement.previous_runtime_epoch, runtime.runtime_epoch);
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&host.paths.worker_launch(manifest.terminal_id)).unwrap(),
        replacement.launch
    );
    let retired: crate::worker::StartupCheckpointEvidence =
        read_json(&host.paths.worker_startup_checkpoint(manifest.terminal_id)).unwrap();
    assert_eq!(
        retired.status,
        crate::worker::StartupStatus::HostRetired,
        "{}",
        first.message
    );
    assert_eq!(
        retired.successor,
        Some(checkpoint_successor_fence(&replacement.launch))
    );

    let second = host
        .resume_session_locked(manifest.terminal_id, ResumeSelection::ExplicitRecreate)
        .await
        .unwrap_err()
        .into_explicit_error();
    assert_eq!(
        second.code,
        crate::protocol::error_code::WORKER_START_FAILED
    );
    assert_eq!(second.message, WORKER_START_PUBLIC_DETAIL);
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(manifest.terminal_id))
            .unwrap(),
        replacement
    );
    assert_eq!(
        read_json::<crate::worker::StartupCheckpointEvidence>(
            &host.paths.worker_startup_checkpoint(manifest.terminal_id)
        )
        .unwrap(),
        retired
    );
}

#[tokio::test]
async fn live_failed_resume_cannot_rearm_or_replace_authority() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let transaction = recreate_survivor_transaction(&host, &manifest, Uuid::new_v4());
    let mut child = spawn_resume_test_child();
    let runtime = planned_resume_runtime(
        &host,
        &transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        child.id(),
        process::process_start_time(child.id()).unwrap(),
        BootstrapDelivery::Failed,
    );
    persist_survivor_resume(&host, &manifest, &transaction, &runtime);
    let checkpoint = persist_failed_resume_checkpoint(&host, &transaction, &runtime);

    assert!(
        host.dead_resume_terminal_evidence(&manifest, &transaction)
            .unwrap()
            .is_none()
    );
    assert!(
        host.resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await
        .is_err()
    );
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(manifest.terminal_id))
            .unwrap(),
        transaction
    );
    assert_eq!(
        read_json::<crate::worker::StartupCheckpointEvidence>(
            &host.paths.worker_startup_checkpoint(manifest.terminal_id)
        )
        .unwrap(),
        checkpoint
    );
    stop_resume_test_child(&mut child);
}

include!("resume_failed_rearm_crash_reopen.rs");
include!("resume_worker_start_diagnostics_adversarial.rs");
