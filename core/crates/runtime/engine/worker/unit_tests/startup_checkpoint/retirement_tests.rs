use super::retirement_test_support::*;
use super::*;

#[test]
fn absent_checkpoint_requires_a_valid_successor_schema() {
    let fixture = CheckpointFixture::new();
    assert_eq!(
        worker_startup_checkpoint_replay_disposition(&fixture.paths, &fixture.launch).unwrap(),
        WorkerStartupCheckpointReplayDisposition::Absent
    );
    assert_eq!(
        prepare(&fixture, &fixture.launch, None).unwrap(),
        WorkerStartupCheckpointPreparation::Absent
    );
    let mut invalid = fixture.launch.clone();
    invalid.schema_version += UNSUPPORTED_SCHEMA_VERSION_OFFSET;
    assert!(prepare(&fixture, &invalid, None).is_err());
    assert!(
        ActiveStartupCheckpoint::begin(
            &fixture.namespace,
            &fixture.paths,
            &invalid,
            fixture.worker,
        )
        .is_err()
    );
    assert!(!fixture.path().exists());
}

#[test]
fn same_fence_retry_is_durably_authorized_and_replayable() {
    let fixture = CheckpointFixture::new();
    ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap();
    assert_eq!(
        worker_startup_checkpoint_replay_disposition(&fixture.paths, &fixture.launch).unwrap(),
        WorkerStartupCheckpointReplayDisposition::Conflicting
    );

    assert_eq!(
        prepare(&fixture, &fixture.launch, None).unwrap(),
        WorkerStartupCheckpointPreparation::Prepared
    );
    assert_eq!(
        worker_startup_checkpoint_replay_disposition(&fixture.paths, &fixture.launch).unwrap(),
        WorkerStartupCheckpointReplayDisposition::Authorized
    );
    let retired = fixture.read();
    assert_eq!(retired.status, StartupStatus::HostRetired);
    assert_eq!(
        retired.successor,
        Some(StartupLaunchFence::from_launch(&fixture.launch).unwrap())
    );
    assert_eq!(
        prepare(&fixture, &fixture.launch, None).unwrap(),
        WorkerStartupCheckpointPreparation::AlreadyPrepared
    );

    let active = ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        fixture.worker,
    )
    .unwrap();
    assert_eq!(active.evidence.status, StartupStatus::Active);
    assert!(active.evidence.successor.is_none());
}

#[test]
fn activated_checkpoint_authorizes_only_its_fenced_resume_successor() {
    let fixture = CheckpointFixture::new();
    let predecessor = activated_runtime(&fixture);
    let successor = successor_for(&fixture);

    assert_eq!(
        prepare(&fixture, &successor, Some(&predecessor)).unwrap(),
        WorkerStartupCheckpointPreparation::Prepared
    );
    assert!(
        ActiveStartupCheckpoint::begin(
            &fixture.namespace,
            &fixture.paths,
            &fixture.launch,
            fixture.worker,
        )
        .is_err()
    );
    crate::paths::atomic_write_json(
        &fixture.paths.worker_launch(successor.terminal_id),
        &successor,
    )
    .unwrap();
    let active = ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &successor,
        fixture.worker,
    )
    .unwrap();
    assert_eq!(active.evidence.runtime_epoch, successor.runtime_epoch);
    assert_eq!(active.evidence.worker_id, successor.worker_id);
}

#[test]
fn settled_checkpoint_still_requires_successor_authorization_before_begin() {
    let fixture = CheckpointFixture::new();
    ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap();
    super::super::settlement::settle_worker_startup_checkpoint_if_gone(
        &fixture.paths,
        &fixture.read(),
    )
    .unwrap();
    assert!(
        ActiveStartupCheckpoint::begin(
            &fixture.namespace,
            &fixture.paths,
            &fixture.launch,
            fixture.worker,
        )
        .is_err()
    );
    assert_eq!(
        prepare(&fixture, &fixture.launch, None).unwrap(),
        WorkerStartupCheckpointPreparation::Prepared
    );
}

#[test]
fn settled_pre_runtime_attempt_authorizes_a_rotated_creation_successor() {
    let fixture = CheckpointFixture::new();
    ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap();
    super::super::settlement::settle_worker_startup_checkpoint_if_gone(
        &fixture.paths,
        &fixture.read(),
    )
    .unwrap();
    let mut successor = successor_for(&fixture);
    successor.controller_generation -= STALE_IDENTITY_OFFSET;
    assert!(prepare(&fixture, &successor, None).is_err());
    successor.controller_generation = fixture.launch.controller_generation;

    assert_eq!(
        prepare(&fixture, &successor, None).unwrap(),
        WorkerStartupCheckpointPreparation::Prepared
    );
    assert_eq!(
        fixture.read().successor,
        Some(StartupLaunchFence::from_launch(&successor).unwrap())
    );
    assert_eq!(
        settle_worker_startup_checkpoint_for_launch(
            &fixture.paths,
            &successor,
            WorkerStartupCheckpointPolicy::SettlePresent,
        )
        .unwrap(),
        WorkerStartupCheckpointSettlement::AlreadySettled
    );
    let changed = successor_for(&fixture);
    assert!(
        settle_worker_startup_checkpoint_for_launch(
            &fixture.paths,
            &changed,
            WorkerStartupCheckpointPolicy::SettlePresent,
        )
        .is_err()
    );
}
