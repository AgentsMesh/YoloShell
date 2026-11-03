use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn creation_checkpoint_runtime_replacement_blocks_retirement() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let checkpoint = fixture.checkpoint(
        crate::worker::StartupStage::RuntimeActivation,
        crate::worker::StartupPhase::Completed,
        crate::worker::StartupStatus::RuntimeActivated,
        Some(checkpoint_process(CHECKPOINT_TEST_CHILD_PID)),
    );
    fixture.write(&checkpoint);
    fixture.write_runtime(&fixture.predecessor(BootstrapDelivery::NotRequired));
    let successor = fixture.publish_successor();
    let replacement_path = fixture
        .host
        .paths
        .runtime_record(fixture.binding.result.terminal_id);
    let mut replacement = fixture.predecessor(BootstrapDelivery::NotRequired);
    replacement.runtime_epoch = Uuid::new_v4();
    let expected_replacement = replacement.clone();
    let _hook = creation_checkpoint_test_hook::install(move || {
        atomic_write_json(&replacement_path, &replacement)
    });

    assert!(
        fixture
            .host
            .prepare_worker_startup_checkpoint_before_spawn(&fixture.manifest(), &successor)
            .is_err()
    );
    assert_eq!(fixture.read(), checkpoint);
    assert_eq!(
        read_json::<RuntimeRecord>(
            &fixture
                .host
                .paths
                .runtime_record(fixture.binding.result.terminal_id)
        )
        .unwrap(),
        expected_replacement
    );
}

#[tokio::test]
async fn creation_checkpoint_accepts_adopted_predecessor_generation() {
    let mut fixture = CreationCheckpointFixture::new(true).await;
    let checkpoint = fixture.checkpoint(
        crate::worker::StartupStage::RuntimeActivation,
        crate::worker::StartupPhase::Completed,
        crate::worker::StartupStatus::RuntimeActivated,
        Some(checkpoint_process(CHECKPOINT_TEST_CHILD_PID)),
    );
    fixture.write(&checkpoint);
    fixture.host.controller_generation = fixture
        .host
        .controller_generation
        .checked_add(CHECKPOINT_TEST_IDENTITY_OFFSET)
        .unwrap();
    let mut predecessor = fixture.predecessor(BootstrapDelivery::NotRequired);
    predecessor.controller_generation = fixture.host.controller_generation;
    fixture.write_runtime(&predecessor);
    let successor = fixture.publish_successor();
    let _gone = gone_checkpoint_processes();

    let prepared = fixture
        .host
        .prepare_worker_startup_checkpoint_before_spawn(&fixture.manifest(), &successor)
        .unwrap();

    assert_eq!(
        prepared,
        crate::worker::WorkerStartupCheckpointPreparation::Prepared
    );
    let retired = fixture.read();
    assert_eq!(retired.status, crate::worker::StartupStatus::HostRetired);
    assert_eq!(
        retired.successor,
        Some(checkpoint_successor_fence(&successor))
    );
}

#[tokio::test]
async fn creation_checkpoint_spawn_failed_rotation_is_replayable() {
    let mut fixture = CreationCheckpointFixture::new(true).await;
    let checkpoint = fixture.active_checkpoint();
    fixture.write(&checkpoint);
    lock_recover(&fixture.host.creation_capabilities)
        .set_active_phase(
            &fixture.ledger_path,
            fixture.binding.result,
            CreationPhase::SpawnFailed,
        )
        .unwrap();
    fixture.host.runtime_executable = fixture.missing_runtime();
    fixture.host.worker_ready_timeout = creation_launch_test_timeout(HOST_BLOCKING_TEST_TIMEOUT);
    let previous = fixture.planned.clone();
    let _gone = gone_checkpoint_processes();

    let first = fixture
        .host
        .create_session(fixture.request.clone())
        .await
        .unwrap_err();
    let durable = CreationLedger::open(&fixture.ledger_path).unwrap();
    let binding = durable
        .bindings
        .iter()
        .find(|value| value.result == fixture.binding.result)
        .unwrap();
    let successor_identity = binding.launch.as_ref().unwrap();
    assert_eq!(binding.phase, CreationPhase::Launching);
    assert_rotated_launch_authority(&previous, successor_identity);
    let successor: WorkerLaunchSpec = read_json(
        &fixture
            .host
            .paths
            .worker_launch(fixture.binding.result.terminal_id),
    )
    .unwrap();
    assert_eq!(
        fixture.read().successor,
        Some(checkpoint_successor_fence(&successor))
    );
    assert_eq!(
        fixture.read().status,
        crate::worker::StartupStatus::HostRetired
    );
    assert_eq!(first.code, crate::protocol::error_code::WORKER_START_FAILED);

    let terminal_id = fixture.binding.result.terminal_id;
    let checkpoint_after_first = fixture.read();
    let request = fixture.request.clone();
    let ledger_path = fixture.ledger_path.clone();
    let mut reopen_config = HostConfig::new(fixture._directory.path());
    reopen_config.runtime_executable = Some(fixture.missing_runtime());
    drop(fixture.host);
    let mut reopened = Host::open(reopen_config).unwrap();
    reopened.worker_ready_timeout = creation_launch_test_timeout(HOST_BLOCKING_TEST_TIMEOUT);
    assert!(reopened.controller_generation > successor.controller_generation);

    let second = reopened.create_session(request).await.unwrap_err();
    assert_eq!(
        second.code,
        crate::protocol::error_code::WORKER_START_FAILED
    );
    let replayed: WorkerLaunchSpec = read_json(&reopened.paths.worker_launch(terminal_id)).unwrap();
    assert_eq!(replayed, successor);
    let replayed_checkpoint: crate::worker::StartupCheckpointEvidence =
        read_json(&reopened.paths.worker_startup_checkpoint(terminal_id)).unwrap();
    assert_eq!(replayed_checkpoint, checkpoint_after_first);
    assert_eq!(
        replayed_checkpoint.successor,
        Some(checkpoint_successor_fence(&successor))
    );
    let replayed_ledger = CreationLedger::open(&ledger_path).unwrap();
    let replayed_binding = replayed_ledger
        .bindings
        .iter()
        .find(|value| value.result.terminal_id == terminal_id)
        .unwrap();
    assert_eq!(replayed_binding.phase, CreationPhase::Launching);
    assert_eq!(replayed_binding.launch.as_ref(), Some(successor_identity));
}

#[tokio::test]
async fn creation_checkpoint_absent_old_generation_launch_is_replayable() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let binding = fixture.binding.clone();
    let planned = fixture.planned.clone();
    let launch = fixture.launch.clone();
    let checkpoint_path = fixture.path();
    let reopen_config = HostConfig::new(fixture._directory.path());
    assert!(!checkpoint_path.exists());
    drop(fixture.host);

    let reopened = Host::open(reopen_config).unwrap();
    assert!(reopened.controller_generation > launch.controller_generation);
    assert!(
        reopened
            .reconcile_pre_runtime_checkpoint(&binding, &planned)
            .unwrap()
    );
    assert_eq!(
        reopened
            .prepare_creation_worker_launch(&binding, &planned)
            .unwrap(),
        launch
    );
    assert!(!checkpoint_path.exists());
}
