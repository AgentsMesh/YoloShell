use super::*;

#[test]
fn live_worker_custody_cannot_be_marked_host_settled() {
    let fixture = CheckpointFixture::new();
    let evidence = fixture.begin().evidence;
    assert!(settle_for_launch(&fixture).is_err());
    assert_eq!(fixture.read(), evidence);
}

#[test]
fn conclusively_gone_custody_requires_successor_authorization_to_rearm() {
    let fixture = CheckpointFixture::new();
    let gone = gone_process();
    ActiveStartupCheckpoint::begin(&fixture.namespace, &fixture.paths, &fixture.launch, gone)
        .unwrap();

    assert_eq!(
        settle_for_launch(&fixture).unwrap(),
        WorkerStartupCheckpointSettlement::Settled
    );
    let settled = fixture.read();
    assert_eq!(settled.status, StartupStatus::HostSettled);
    assert_eq!(
        settled.worker,
        StartupProcessIdentity::new(gone.pid, gone.start_time)
    );
    assert_eq!(
        settle_for_launch(&fixture).unwrap(),
        WorkerStartupCheckpointSettlement::AlreadySettled
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
    assert_eq!(
        prepare_worker_startup_checkpoint_for_launch(&fixture.paths, &fixture.launch, None)
            .unwrap(),
        WorkerStartupCheckpointPreparation::Prepared
    );
    let rearmed = ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        fixture.worker,
    )
    .unwrap();
    assert_eq!(rearmed.evidence.status, StartupStatus::Active);
    assert_eq!(
        fixture.read().worker,
        StartupProcessIdentity::new(fixture.worker.pid, fixture.worker.start_time)
    );
}

#[test]
fn ambiguous_unbound_child_custody_cannot_be_settled() {
    let fixture = CheckpointFixture::new();
    let mut checkpoint = ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap();
    for stage in STARTUP_STAGES.into_iter().take(CHILD_BINDING_STAGE_COUNT) {
        checkpoint
            .enter(&fixture.namespace, &fixture.paths, stage)
            .unwrap();
        if stage != StartupStage::ChildProcess {
            checkpoint
                .complete(&fixture.namespace, &fixture.paths, stage)
                .unwrap();
        }
    }
    let expected = fixture.read();

    assert!(settle_for_launch(&fixture).is_err());
    assert_eq!(fixture.read(), expected);
}

#[test]
fn live_bound_child_custody_cannot_be_settled() {
    let fixture = CheckpointFixture::new();
    let mut checkpoint = ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap();
    for stage in STARTUP_STAGES.into_iter().take(CHILD_BINDING_STAGE_COUNT) {
        checkpoint
            .enter(&fixture.namespace, &fixture.paths, stage)
            .unwrap();
        if stage == StartupStage::ChildProcess {
            checkpoint
                .complete_child(&fixture.namespace, &fixture.paths, fixture.worker)
                .unwrap();
        } else {
            checkpoint
                .complete(&fixture.namespace, &fixture.paths, stage)
                .unwrap();
        }
    }
    let expected = fixture.read();

    assert!(settle_for_launch(&fixture).is_err());
    assert_eq!(fixture.read(), expected);
}

#[test]
fn activated_runtime_can_be_settled_after_all_custody_is_gone() {
    let fixture = CheckpointFixture::new();
    let worker = gone_process();
    let child = gone_process();
    let mut checkpoint =
        ActiveStartupCheckpoint::begin(&fixture.namespace, &fixture.paths, &fixture.launch, worker)
            .unwrap();
    for stage in STARTUP_STAGES {
        checkpoint
            .enter(&fixture.namespace, &fixture.paths, stage)
            .unwrap();
        if stage == StartupStage::RuntimeActivation {
            checkpoint
                .retire_runtime_activated(&fixture.namespace, &fixture.paths)
                .unwrap();
        } else if stage == StartupStage::ChildProcess {
            checkpoint
                .complete_child(&fixture.namespace, &fixture.paths, child)
                .unwrap();
        } else {
            checkpoint
                .complete(&fixture.namespace, &fixture.paths, stage)
                .unwrap();
        }
    }
    assert_eq!(
        settle_for_launch(&fixture).unwrap(),
        WorkerStartupCheckpointSettlement::Settled
    );
    assert_eq!(fixture.read().status, StartupStatus::HostSettled);
}

#[test]
fn stale_settlement_evidence_cannot_retire_a_replacement() {
    let fixture = CheckpointFixture::new();
    let expected = ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap()
    .evidence;
    let mut replacement = expected.clone();
    replacement.controller_generation += STALE_IDENTITY_OFFSET;
    crate::paths::atomic_write_json(&fixture.path(), &replacement).unwrap();

    assert!(
        super::super::settlement::settle_worker_startup_checkpoint_if_gone(
            &fixture.paths,
            &expected,
        )
        .is_err()
    );
    assert_eq!(fixture.read(), replacement);
}

fn settle_for_launch(
    fixture: &CheckpointFixture,
) -> anyhow::Result<WorkerStartupCheckpointSettlement> {
    settle_worker_startup_checkpoint_for_launch(
        &fixture.paths,
        &fixture.launch,
        WorkerStartupCheckpointPolicy::SettlePresent,
    )
}
