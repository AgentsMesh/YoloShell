use super::*;

#[test]
fn every_stage_persists_unambiguous_entered_and_completed_evidence() {
    let fixture = CheckpointFixture::new();
    let mut checkpoint = fixture.begin();
    let initial = fixture.read();
    assert_eq!(initial.stage, StartupStage::LaunchContract);
    assert_eq!(initial.phase, StartupPhase::Completed);
    assert_eq!(initial.status, StartupStatus::Active);
    assert_eq!(
        initial.worker,
        StartupProcessIdentity::new(fixture.worker.pid, fixture.worker.start_time)
    );
    assert!(initial.child.is_none());

    for stage in STARTUP_STAGES {
        checkpoint
            .enter(&fixture.namespace, &fixture.paths, stage)
            .unwrap();
        let entered = fixture.read();
        assert_eq!(
            (entered.stage, entered.phase),
            (stage, StartupPhase::Entered)
        );
        assert_eq!(entered.status, StartupStatus::Active);
        if stage == StartupStage::RuntimeActivation {
            checkpoint
                .retire_runtime_activated(&fixture.namespace, &fixture.paths)
                .unwrap();
        } else if stage == StartupStage::ChildProcess {
            checkpoint
                .complete_child(&fixture.namespace, &fixture.paths, fixture.child())
                .unwrap();
        } else {
            checkpoint
                .complete(&fixture.namespace, &fixture.paths, stage)
                .unwrap();
        }
        let completed = fixture.read();
        assert_eq!(
            (completed.stage, completed.phase),
            (stage, StartupPhase::Completed)
        );
    }

    let retired = fixture.read();
    assert_eq!(retired.status, StartupStatus::RuntimeActivated);
    assert_eq!(
        retired.child,
        Some(StartupProcessIdentity::new(
            fixture.child().pid,
            fixture.child().start_time
        ))
    );
}

#[test]
fn skipped_regressed_duplicate_and_incomplete_transitions_fail_closed() {
    let fixture = CheckpointFixture::new();
    let mut checkpoint = fixture.begin();
    assert!(
        checkpoint
            .enter(
                &fixture.namespace,
                &fixture.paths,
                StartupStage::WorkerSocket
            )
            .is_err()
    );
    checkpoint
        .enter(
            &fixture.namespace,
            &fixture.paths,
            StartupStage::StartupContainment,
        )
        .unwrap();
    assert!(
        checkpoint
            .enter(
                &fixture.namespace,
                &fixture.paths,
                StartupStage::StartupContainment,
            )
            .is_err()
    );
    assert!(
        checkpoint
            .complete(
                &fixture.namespace,
                &fixture.paths,
                StartupStage::WorkerSocket,
            )
            .is_err()
    );
    assert_eq!(fixture.read().phase, StartupPhase::Entered);
}

#[test]
fn child_identity_is_bound_only_by_child_completion() {
    let fixture = CheckpointFixture::new();
    let mut checkpoint = fixture.begin();
    for stage in STARTUP_STAGES.into_iter().take(CHILD_BINDING_STAGE_COUNT) {
        checkpoint
            .enter(&fixture.namespace, &fixture.paths, stage)
            .unwrap();
        if stage == StartupStage::ChildProcess {
            assert!(
                checkpoint
                    .complete(&fixture.namespace, &fixture.paths, stage)
                    .is_err()
            );
            checkpoint
                .complete_child(&fixture.namespace, &fixture.paths, fixture.child())
                .unwrap();
        } else {
            checkpoint
                .complete(&fixture.namespace, &fixture.paths, stage)
                .unwrap();
        }
    }
    assert_eq!(
        fixture.read().child,
        Some(StartupProcessIdentity::new(
            fixture.child().pid,
            fixture.child().start_time
        ))
    );
}

#[tokio::test]
async fn pre_runtime_failure_retains_the_exact_entered_stage() {
    let fixture = CheckpointFixture::new();
    crate::worker::tests::write_launch(&fixture.paths, fixture.launch.terminal_id, &fixture.launch);
    let journal_path = fixture.paths.journal(fixture.launch.terminal_id);
    std::fs::create_dir(&journal_path).unwrap();

    assert!(
        crate::worker::run(fixture.paths.clone(), fixture.launch.terminal_id)
            .await
            .is_err()
    );
    assert!(fixture.path().is_file());
    assert!(journal_path.is_dir());
    let evidence = fixture.read();
    assert_eq!(evidence.stage, StartupStage::TerminalJournal);
    assert_eq!(evidence.phase, StartupPhase::Entered);
    assert_eq!(evidence.status, StartupStatus::Active);
    assert!(evidence.child.is_none());
    assert!(
        !fixture
            .paths
            .runtime_record(fixture.launch.terminal_id)
            .exists()
    );
}

#[test]
fn activation_transfer_does_not_run_when_checkpoint_cas_fails() {
    let fixture = CheckpointFixture::new();
    let mut checkpoint = fixture.begin();
    for stage in STARTUP_STAGES {
        checkpoint
            .enter(&fixture.namespace, &fixture.paths, stage)
            .unwrap();
        if stage == StartupStage::RuntimeActivation {
            break;
        }
        if stage == StartupStage::ChildProcess {
            checkpoint
                .complete_child(&fixture.namespace, &fixture.paths, fixture.child())
                .unwrap();
        } else {
            checkpoint
                .complete(&fixture.namespace, &fixture.paths, stage)
                .unwrap();
        }
    }
    let mut raced = fixture.read();
    raced.controller_generation += STALE_IDENTITY_OFFSET;
    crate::paths::atomic_write_json(&fixture.path(), &raced).unwrap();
    let activated = std::cell::Cell::new(false);

    assert!(
        crate::worker::activate_after_durable_checkpoint(
            &mut checkpoint,
            &fixture.namespace,
            &fixture.paths,
            || activated.set(true),
        )
        .is_err()
    );
    assert!(!activated.get());
    assert_eq!(fixture.read(), raced);
}
