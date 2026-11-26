use super::*;

#[test]
fn replay_disposition_distinguishes_every_durable_state() {
    let fixture = CheckpointFixture::new();
    assert_eq!(
        worker_startup_checkpoint_replay_disposition(&fixture.paths, &fixture.launch).unwrap(),
        WorkerStartupCheckpointReplayDisposition::Absent
    );
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
    super::super::settlement::settle_worker_startup_checkpoint_if_gone(
        &fixture.paths,
        &fixture.read(),
    )
    .unwrap();
    assert_eq!(
        worker_startup_checkpoint_replay_disposition(&fixture.paths, &fixture.launch).unwrap(),
        WorkerStartupCheckpointReplayDisposition::PreparationRequired
    );
    prepare_worker_startup_checkpoint_for_launch(&fixture.paths, &fixture.launch, None).unwrap();
    assert_eq!(
        worker_startup_checkpoint_replay_disposition(&fixture.paths, &fixture.launch).unwrap(),
        WorkerStartupCheckpointReplayDisposition::Authorized
    );
    let mut changed = fixture.launch.clone();
    changed.command = TEST_CWD.into();
    assert_eq!(
        worker_startup_checkpoint_replay_disposition(&fixture.paths, &changed).unwrap(),
        WorkerStartupCheckpointReplayDisposition::Conflicting
    );
}

#[test]
fn replay_disposition_rejects_malformed_evidence() {
    let fixture = CheckpointFixture::new();
    fixture.write_bytes(
        TEST_INVALID_CHECKPOINT_JSON,
        u64::try_from(TEST_INVALID_CHECKPOINT_JSON.len()).unwrap(),
    );
    assert!(worker_startup_checkpoint_replay_disposition(&fixture.paths, &fixture.launch).is_err());
}
