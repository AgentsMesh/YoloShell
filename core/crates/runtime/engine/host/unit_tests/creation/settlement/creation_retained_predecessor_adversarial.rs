use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn creation_retained_predecessor_runtime_replays_retired_successor_after_reopen() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let checkpoint = fixture.checkpoint(
        crate::worker::StartupStage::RuntimeActivation,
        crate::worker::StartupPhase::Completed,
        crate::worker::StartupStatus::RuntimeActivated,
        Some(checkpoint_process(CHECKPOINT_TEST_CHILD_PID)),
    );
    let predecessor = fixture.predecessor(BootstrapDelivery::Failed);
    fixture.write(&checkpoint);
    fixture.write_runtime(&predecessor);
    lock_recover(&fixture.host.creation_capabilities)
        .set_active_phase(
            &fixture.ledger_path,
            fixture.binding.result,
            CreationPhase::SpawnFailed,
        )
        .unwrap();
    let successor_identity = lock_recover(&fixture.host.creation_capabilities)
        .prepare_launch(&fixture.ledger_path, fixture.binding.result)
        .unwrap();
    let successor_binding = lock_recover(&fixture.host.creation_capabilities)
        .bindings
        .iter()
        .find(|value| value.result == fixture.binding.result)
        .unwrap()
        .clone();
    let successor = planned_worker_launch(&fixture.host, &successor_binding, &successor_identity);
    fixture.host.publish_worker_launch(&successor).unwrap();
    let _gone = gone_checkpoint_processes();
    assert_eq!(
        fixture
            .host
            .prepare_worker_startup_checkpoint_before_spawn(&fixture.manifest(), &successor)
            .unwrap(),
        crate::worker::WorkerStartupCheckpointPreparation::Prepared
    );
    let retired = fixture.read();
    assert_eq!(retired.status, crate::worker::StartupStatus::HostRetired);
    assert_eq!(
        retired.successor,
        Some(checkpoint_successor_fence(&successor))
    );

    let request = fixture.request.clone();
    let terminal_id = fixture.binding.result.terminal_id;
    let ledger_path = fixture.ledger_path.clone();
    let missing_runtime = fixture.missing_runtime();
    let mut reopen_config = HostConfig::new(fixture._directory.path());
    reopen_config.runtime_executable = Some(missing_runtime);
    drop(fixture.host);
    let mut reopened = Host::open(reopen_config).unwrap();
    reopened.worker_ready_timeout = creation_launch_test_timeout(HOST_BLOCKING_TEST_TIMEOUT);

    assert!(
        reopened
            .reconcile_pre_runtime_checkpoint(&successor_binding, &successor_identity)
            .unwrap()
    );
    let error = reopened.create_session(request).await.unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&reopened.paths.worker_launch(terminal_id)).unwrap(),
        successor
    );
    assert_eq!(
        read_json::<RuntimeRecord>(&reopened.paths.runtime_record(terminal_id)).unwrap(),
        predecessor
    );
    assert_eq!(
        read_json::<crate::worker::StartupCheckpointEvidence>(
            &reopened.paths.worker_startup_checkpoint(terminal_id)
        )
        .unwrap(),
        retired
    );
    let durable = CreationLedger::open(&ledger_path).unwrap();
    let binding = durable
        .bindings
        .iter()
        .find(|value| value.result.terminal_id == terminal_id)
        .unwrap();
    assert_eq!(binding.phase, CreationPhase::Launching);
    assert_eq!(binding.launch.as_ref(), Some(&successor_identity));
}

include!("creation_retained_predecessor_recovery_tests.rs");
