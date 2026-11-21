use super::{constants::*, fixture::*};
use crate::{FreshInitialization, JournalError};
use yolo_terminal_model::{RuntimeRecord, TerminalManifest, WorkerLaunchSpec};
use yolo_startup_checkpoint_contract::{
    StartupCheckpointEvidence, StartupPhase, StartupStage, StartupStatus,
};

#[test]
fn only_the_active_pre_ready_runtime_activation_checkpoint_can_initialize() {
    for (stage, phase, status) in [
        (
            StartupStage::BootstrapInput,
            StartupPhase::Completed,
            StartupStatus::Active,
        ),
        (
            StartupStage::RuntimeActivation,
            StartupPhase::Completed,
            StartupStatus::RuntimeActivated,
        ),
        (
            StartupStage::RuntimeActivation,
            StartupPhase::Entered,
            StartupStatus::HostSettled,
        ),
    ] {
        let fixture = Fixture::new();
        let path = fixture
            .paths
            .worker_startup_checkpoint(fixture.runtime.terminal_id);
        let mut checkpoint: StartupCheckpointEvidence = fixture.read(&path);
        checkpoint.stage = stage;
        checkpoint.phase = phase;
        checkpoint.status = status;
        fixture.write(&path, &checkpoint);
        assert!(matches!(fixture.proof(), Err(JournalError::ScopeMismatch)));
        assert!(!fixture.anchor().exists());
        assert!(!fixture.journal().exists());
    }
}

#[test]
fn archived_manifest_and_mismatched_launch_or_runtime_never_produce_fresh_authority() {
    let fixture = Fixture::new();
    let path = fixture.paths.manifest(fixture.runtime.terminal_id);
    let mut manifest: TerminalManifest = fixture.read(&path);
    manifest.archived = true;
    fixture.write(&path, &manifest);
    assert!(matches!(fixture.proof(), Err(JournalError::ScopeMismatch)));
    let fixture = Fixture::new();
    let path = fixture.paths.worker_launch(fixture.runtime.terminal_id);
    let mut launch: WorkerLaunchSpec = fixture.read(&path);
    launch.auth_token = WRONG_SCOPE.into();
    fixture.write(&path, &launch);
    assert!(matches!(fixture.proof(), Err(JournalError::ScopeMismatch)));
    let fixture = Fixture::new();
    let mut current = fixture.runtime.clone();
    current.worker_start_time += STEP;
    assert!(matches!(
        FreshInitialization::capture(fixture.session(), &fixture.paths, &current),
        Err(JournalError::ScopeMismatch)
    ));
}

#[test]
fn checkpoint_is_bound_to_both_exact_worker_and_child_identity() {
    let fixture = Fixture::new();
    let path = fixture.paths.runtime_record(fixture.runtime.terminal_id);
    let mut durable: RuntimeRecord = fixture.read(&path);
    durable.child_start_time += STEP;
    fixture.write(&path, &durable);
    assert!(matches!(fixture.proof(), Err(JournalError::ScopeMismatch)));
    let fixture = Fixture::new();
    let path = fixture
        .paths
        .worker_startup_checkpoint(fixture.runtime.terminal_id);
    let mut checkpoint: StartupCheckpointEvidence = fixture.read(&path);
    checkpoint.worker.start_time += STEP;
    fixture.write(&path, &checkpoint);
    assert!(matches!(fixture.proof(), Err(JournalError::ScopeMismatch)));
}
