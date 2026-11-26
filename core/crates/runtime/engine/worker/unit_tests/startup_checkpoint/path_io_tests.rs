use super::*;

const CHECKPOINT_GUARD_TEST_ERROR: &str = "checkpoint namespace guard rejected access";

#[test]
fn a_missing_session_directory_is_not_an_absent_checkpoint() {
    let fixture = CheckpointFixture::new();
    std::fs::remove_dir_all(fixture.paths.session_dir(fixture.launch.terminal_id)).unwrap();

    assert!(
        settle_worker_startup_checkpoint_for_launch(
            &fixture.paths,
            &fixture.launch,
            WorkerStartupCheckpointPolicy::RequireAbsent,
        )
        .is_err()
    );
}

#[test]
fn removal_after_open_is_a_race_not_an_absent_checkpoint() {
    let fixture = CheckpointFixture::new();
    fixture.begin();
    let entry = fixture.namespace.file(&fixture.path()).unwrap();

    let result = io::read_optional(entry, || {
        std::fs::remove_file(fixture.path())?;
        Ok(())
    });

    assert!(result.is_err());
}

#[test]
fn missing_checkpoint_requires_a_successful_namespace_guard() {
    let fixture = CheckpointFixture::new();
    let entry = fixture.namespace.file(&fixture.path()).unwrap();

    let result = io::read_optional(entry, || anyhow::bail!(CHECKPOINT_GUARD_TEST_ERROR));

    assert!(result.is_err());
}
