use crate::host::tests::*;
use super::*;

const CHECKPOINT_SPAWN_TEST_TIMEOUT: Duration = Duration::from_secs(2);

#[tokio::test]
async fn session_spawn_checkpoint_prepares_runtime_activated_successor() {
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
    let manifest = fixture.manifest();
    let _gone = gone_checkpoint_processes();

    let prepared = fixture
        .host
        .prepare_worker_startup_checkpoint_before_spawn(&manifest, &successor)
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
    assert_eq!(
        fixture
            .host
            .prepare_worker_startup_checkpoint_before_spawn(&manifest, &successor)
            .unwrap(),
        crate::worker::WorkerStartupCheckpointPreparation::AlreadyPrepared
    );
}

#[tokio::test]
async fn session_spawn_checkpoint_rejects_a_live_predecessor() {
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
    let _worker = crate::process::install_test_process_identity_relation(
        CHECKPOINT_TEST_PROCESS_PID,
        CHECKPOINT_TEST_START_TIME,
        crate::process::ProcessIdentityRelation::Matching,
    );

    assert!(
        fixture
            .host
            .prepare_worker_startup_checkpoint_before_spawn(&fixture.manifest(), &successor)
            .is_err()
    );
    assert_eq!(fixture.read(), checkpoint);
}

#[tokio::test]
async fn session_spawn_checkpoint_retires_resolved_failed_bootstrap() {
    let fixture = CreationCheckpointFixture::new(true).await;
    let checkpoint = fixture.checkpoint(
        crate::worker::StartupStage::CommandExec,
        crate::worker::StartupPhase::Entered,
        crate::worker::StartupStatus::Active,
        Some(checkpoint_process(CHECKPOINT_TEST_CHILD_PID)),
    );
    fixture.write(&checkpoint);
    fixture.write_runtime(&fixture.predecessor(BootstrapDelivery::Failed));
    let successor = fixture.publish_successor();
    let _gone = gone_checkpoint_processes();

    assert_eq!(
        fixture
            .host
            .prepare_worker_startup_checkpoint_before_spawn(&fixture.manifest(), &successor)
            .unwrap(),
        crate::worker::WorkerStartupCheckpointPreparation::Prepared
    );
    assert_eq!(
        fixture.read().successor,
        Some(checkpoint_successor_fence(&successor))
    );
}

#[tokio::test]
async fn session_spawn_checkpoint_requires_exact_predecessor_identity() {
    let mutations: [fn(&mut RuntimeRecord); 5] = [
        |value| value.runtime_epoch = Uuid::new_v4(),
        |value| value.worker_id = Uuid::new_v4(),
        |value| value.controller_generation += CHECKPOINT_TEST_IDENTITY_OFFSET,
        |value| value.worker_pid -= 1,
        |value| value.child_pid -= 1,
    ];
    for mutate in mutations {
        let fixture = CreationCheckpointFixture::new(true).await;
        let checkpoint = fixture.checkpoint(
            crate::worker::StartupStage::RuntimeActivation,
            crate::worker::StartupPhase::Completed,
            crate::worker::StartupStatus::RuntimeActivated,
            Some(checkpoint_process(CHECKPOINT_TEST_CHILD_PID)),
        );
        fixture.write(&checkpoint);
        let mut predecessor = fixture.predecessor(BootstrapDelivery::NotRequired);
        mutate(&mut predecessor);
        fixture.write_runtime(&predecessor);
        let successor = fixture.publish_successor();

        assert!(
            fixture
                .host
                .prepare_worker_startup_checkpoint_before_spawn(&fixture.manifest(), &successor)
                .is_err()
        );
        assert_eq!(fixture.read(), checkpoint);
    }
}

#[tokio::test]
async fn session_spawn_checkpoint_preserves_successor_after_spawn_failure() {
    let mut fixture = CreationCheckpointFixture::new(true).await;
    let checkpoint = fixture.checkpoint(
        crate::worker::StartupStage::RuntimeActivation,
        crate::worker::StartupPhase::Completed,
        crate::worker::StartupStatus::RuntimeActivated,
        Some(checkpoint_process(CHECKPOINT_TEST_CHILD_PID)),
    );
    fixture.write(&checkpoint);
    fixture.write_runtime(&fixture.predecessor(BootstrapDelivery::NotRequired));
    let successor = fixture.publish_successor();
    fixture.host.runtime_executable = fixture.missing_runtime();
    let _gone = gone_checkpoint_processes();

    let result = fixture
        .host
        .spawn_published_worker_before(
            &fixture.manifest(),
            &successor,
            UnstartedLaunchPolicy::Retire,
            HostOperationDeadline::after(CHECKPOINT_SPAWN_TEST_TIMEOUT),
        )
        .await;

    assert!(result.is_err());
    assert_eq!(
        read_json::<WorkerLaunchSpec>(
            &fixture
                .host
                .paths
                .worker_launch(fixture.binding.result.terminal_id)
        )
        .unwrap(),
        successor
    );
    assert_eq!(
        fixture.read().status,
        crate::worker::StartupStatus::HostRetired
    );
}
