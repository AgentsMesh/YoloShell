use super::*;

const TEST_PENDING_INTENT: &[u8] = b"pending-intent";
const TEST_PENDING_INTENT_MAX_BYTES: u64 = TEST_PENDING_INTENT.len() as u64;

#[test]
fn pending_intent_blocks_absent_checkpoint_creation_without_disk_change() {
    let fixture = CheckpointFixture::new();
    let evidence = initial_evidence(&fixture);
    let lifecycle = fixture
        .paths
        .try_acquire_session_lifecycle_lock(fixture.launch.terminal_id)
        .unwrap()
        .unwrap();

    let result = io::begin_with_test_hook(&fixture.namespace, &fixture.paths, &evidence, || {
        publish_pending_intent(&fixture)
    });

    drop(lifecycle);
    assert!(result.is_err());
    assert!(!fixture.path().exists());
    assert_eq!(
        std::fs::read(intent_path(&fixture)).unwrap(),
        TEST_PENDING_INTENT
    );
}

#[test]
fn pending_intent_blocks_checkpoint_replacement_without_disk_change() {
    let fixture = CheckpointFixture::new();
    let checkpoint = fixture.begin();
    let current = fixture.read();
    let bytes = std::fs::read(fixture.path()).unwrap();
    let mut next = current.clone();
    next.stage = StartupStage::StartupContainment;
    next.phase = StartupPhase::Entered;

    let result =
        io::replace_with_test_hook(&fixture.namespace, &fixture.paths, &current, &next, || {
            publish_pending_intent(&fixture)
        });

    assert!(result.is_err());
    assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
    assert_eq!(checkpoint.evidence, current);
}

#[test]
fn checkpoint_create_and_replace_succeed_without_an_intent() {
    let fixture = CheckpointFixture::new();
    let lifecycle = fixture
        .paths
        .try_acquire_session_lifecycle_lock(fixture.launch.terminal_id)
        .unwrap()
        .unwrap();
    let mut checkpoint = ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        fixture.worker,
    )
    .unwrap();
    drop(lifecycle);

    checkpoint
        .enter(
            &fixture.namespace,
            &fixture.paths,
            StartupStage::StartupContainment,
        )
        .unwrap();
    assert_eq!(fixture.read().phase, StartupPhase::Entered);
}

#[test]
fn stale_checkpoint_identity_cannot_replace_the_current_record() {
    let fixture = CheckpointFixture::new();
    let checkpoint = fixture.begin().evidence;
    let (_, stale_identity) = path_io::read_path(&fixture.paths, fixture.launch.terminal_id)
        .unwrap()
        .unwrap();
    let mut displaced = checkpoint.clone();
    displaced.controller_generation += STALE_IDENTITY_OFFSET;
    crate::paths::atomic_write_json(&fixture.path(), &displaced).unwrap();
    let bytes = std::fs::read(fixture.path()).unwrap();

    let mut replacement = checkpoint;
    replacement.stage = StartupStage::StartupContainment;
    replacement.phase = StartupPhase::Entered;
    assert!(path_io::replace_path(&fixture.paths, stale_identity, &replacement).is_err());
    assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
}

fn initial_evidence(fixture: &CheckpointFixture) -> StartupCheckpointEvidence {
    let launch = StartupLaunchFence::from_launch(&fixture.launch).unwrap();
    StartupCheckpointEvidence {
        schema_version: constants::STARTUP_CHECKPOINT_SCHEMA_VERSION,
        terminal_id: fixture.launch.terminal_id,
        runtime_epoch: fixture.launch.runtime_epoch,
        worker_id: fixture.launch.worker_id,
        controller_generation: fixture.launch.controller_generation,
        launch_sha256: launch.launch_sha256,
        boot_id: crate::process::boot_id(),
        worker: StartupProcessIdentity::new(fixture.worker.pid, fixture.worker.start_time),
        child: None,
        stage: StartupStage::LaunchContract,
        phase: StartupPhase::Completed,
        status: StartupStatus::Active,
        successor: None,
        predecessor_runtime_sha256: None,
        pending_pty_custody_rollover: None,
    }
}

fn intent_path(fixture: &CheckpointFixture) -> std::path::PathBuf {
    fixture
        .paths
        .reboot_migration_intent(fixture.launch.terminal_id)
}

fn publish_pending_intent(fixture: &CheckpointFixture) -> anyhow::Result<()> {
    fixture
        .namespace
        .file(&intent_path(fixture))?
        .create_bytes_exclusive(TEST_PENDING_INTENT, TEST_PENDING_INTENT_MAX_BYTES)
        .map(|_| ())
}
