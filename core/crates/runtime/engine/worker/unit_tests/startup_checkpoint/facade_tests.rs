use super::*;

#[test]
fn absent_checkpoint_is_ready_for_a_valid_launch() {
    let fixture = CheckpointFixture::new();
    assert_eq!(
        settle_for_launch(
            &fixture,
            fixture.launch.terminal_id,
            fixture.launch.runtime_epoch,
            fixture.launch.worker_id,
            fixture.launch.controller_generation,
        )
        .unwrap(),
        WorkerStartupCheckpointSettlement::Absent
    );
    assert_eq!(
        require_absent(&fixture).unwrap(),
        WorkerStartupCheckpointSettlement::Absent
    );
}

#[test]
fn require_absent_preserves_every_present_checkpoint() {
    let fixture = CheckpointFixture::new();
    ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap();
    let expected = fixture.read();

    assert!(require_absent(&fixture).is_err());
    assert_eq!(fixture.read(), expected);
}

#[test]
fn invalid_launch_fences_fail_even_when_the_checkpoint_is_absent() {
    let fixture = CheckpointFixture::new();
    let launch = &fixture.launch;
    let invalid = [
        (
            Uuid::nil(),
            launch.runtime_epoch,
            launch.worker_id,
            launch.controller_generation,
        ),
        (
            launch.terminal_id,
            Uuid::nil(),
            launch.worker_id,
            launch.controller_generation,
        ),
        (
            launch.terminal_id,
            launch.runtime_epoch,
            Uuid::nil(),
            launch.controller_generation,
        ),
        (
            launch.terminal_id,
            launch.worker_id,
            launch.worker_id,
            launch.controller_generation,
        ),
        (
            launch.terminal_id,
            launch.runtime_epoch,
            launch.worker_id,
            INVALID_CONTROLLER_GENERATION,
        ),
    ];
    for (terminal_id, runtime_epoch, worker_id, generation) in invalid {
        assert!(
            settle_for_launch(&fixture, terminal_id, runtime_epoch, worker_id, generation).is_err()
        );
    }
}

#[test]
fn every_launch_fence_mismatch_is_rejected_and_preserved() {
    let fixture = CheckpointFixture::new();
    ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap();
    settle_exact(&fixture).unwrap();
    let settled = fixture.read();
    let launch = &fixture.launch;
    let mismatches = [
        (
            launch.terminal_id,
            Uuid::new_v4(),
            launch.worker_id,
            launch.controller_generation,
        ),
        (
            launch.terminal_id,
            launch.runtime_epoch,
            Uuid::new_v4(),
            launch.controller_generation,
        ),
        (
            launch.terminal_id,
            launch.runtime_epoch,
            launch.worker_id,
            launch.controller_generation + STALE_IDENTITY_OFFSET,
        ),
    ];
    for (terminal_id, runtime_epoch, worker_id, generation) in mismatches {
        assert!(
            settle_for_launch(&fixture, terminal_id, runtime_epoch, worker_id, generation).is_err()
        );
        assert_eq!(fixture.read(), settled);
    }

    let mut wrong_session = settled.clone();
    wrong_session.terminal_id = Uuid::new_v4();
    crate::paths::atomic_write_json(&fixture.path(), &wrong_session).unwrap();
    assert!(settle_exact(&fixture).is_err());
    assert_eq!(fixture.read(), wrong_session);
}

#[test]
fn malformed_evidence_and_indeterminate_custody_fail_closed() {
    let malformed = CheckpointFixture::new();
    malformed.write_bytes(
        TEST_INVALID_CHECKPOINT_JSON,
        u64::try_from(TEST_INVALID_CHECKPOINT_JSON.len()).unwrap(),
    );
    assert!(settle_exact(&malformed).is_err());
    assert_eq!(
        std::fs::read(malformed.path()).unwrap(),
        TEST_INVALID_CHECKPOINT_JSON
    );

    let unknown = CheckpointFixture::new();
    let evidence = unknown.begin().evidence;
    let _override = crate::process::install_test_process_identity_relation(
        evidence.worker.pid,
        evidence.worker.start_time,
        crate::process::ProcessIdentityRelation::Unknown,
    );
    assert!(settle_exact(&unknown).is_err());
    assert_eq!(unknown.read(), evidence);
}

fn settle_exact(fixture: &CheckpointFixture) -> anyhow::Result<WorkerStartupCheckpointSettlement> {
    settle_for_launch(
        fixture,
        fixture.launch.terminal_id,
        fixture.launch.runtime_epoch,
        fixture.launch.worker_id,
        fixture.launch.controller_generation,
    )
}

fn settle_for_launch(
    fixture: &CheckpointFixture,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    worker_id: Uuid,
    controller_generation: u64,
) -> anyhow::Result<WorkerStartupCheckpointSettlement> {
    let mut launch = fixture.launch.clone();
    launch.terminal_id = terminal_id;
    launch.runtime_epoch = runtime_epoch;
    launch.worker_id = worker_id;
    launch.controller_generation = controller_generation;
    settle_worker_startup_checkpoint_for_launch(
        &fixture.paths,
        &launch,
        WorkerStartupCheckpointPolicy::SettlePresent,
    )
}

fn require_absent(
    fixture: &CheckpointFixture,
) -> anyhow::Result<WorkerStartupCheckpointSettlement> {
    settle_worker_startup_checkpoint_for_launch(
        &fixture.paths,
        &fixture.launch,
        WorkerStartupCheckpointPolicy::RequireAbsent,
    )
}
