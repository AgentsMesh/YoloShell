use crate::fixture::*;
use yolo_agent_control_runtime::*;
use yolo_startup_checkpoint_contract::{
    StartupCheckpointEvidence,
};

const UNSUPPORTED_CHECKPOINT_SCHEMA: u16 = 4;

#[test]
fn every_retained_startup_artifact_is_pinned_even_when_replacement_has_equal_content() {
    for select in [
        yolo_paths::Paths::runtime_record,
        yolo_paths::Paths::worker_launch,
        yolo_paths::Paths::worker_startup_checkpoint,
        yolo_paths::Paths::manifest,
    ] {
        let fixture = Fixture::new();
        let proof = fixture.proof().unwrap();
        let path = select(&fixture.paths, fixture.runtime.terminal_id);
        let value: serde_json::Value = fixture.read(&path);
        fixture.write(&path, &value);
        assert!(matches!(
            ControlRuntime::initialize_fresh(proof),
            Err(JournalError::Unavailable)
        ));
        assert!(!fixture.anchor().exists());
        assert!(!fixture.journal().exists());
    }
}

#[test]
fn legacy_or_missing_checkpoint_cannot_initialize_and_reopen_never_falls_back() {
    let fixture = Fixture::new();
    let checkpoint_path = fixture
        .paths
        .worker_startup_checkpoint(fixture.runtime.terminal_id);
    let mut checkpoint: StartupCheckpointEvidence = fixture.read(&checkpoint_path);
    checkpoint.schema_version = UNSUPPORTED_CHECKPOINT_SCHEMA;
    fixture.write(&checkpoint_path, &checkpoint);
    assert!(matches!(fixture.proof(), Err(JournalError::ScopeMismatch)));
    fixture.namespace().remove_file(&checkpoint_path).unwrap();
    assert!(matches!(fixture.proof(), Err(JournalError::Unavailable)));
    assert!(matches!(
        fixture.reopen(&fixture.runtime),
        Err(JournalError::Unavailable)
    ));
    assert!(!fixture.anchor().exists());
    assert!(!fixture.journal().exists());
}

#[test]
fn a_second_initializer_cannot_overwrite_the_first_anchor_or_receipts() {
    let fixture = Fixture::new();
    let first = fixture.proof().unwrap();
    let second = fixture.proof().unwrap();
    let owner = ControlRuntime::initialize_fresh(first).unwrap();
    let original_anchor = std::fs::read(fixture.anchor()).unwrap();
    let original_receipts = std::fs::read(
        fixture
            .journal()
            .join(yolo_agent_control_journal::JOURNAL_FILE),
    )
    .unwrap();
    assert!(matches!(
        ControlRuntime::initialize_fresh(second),
        Err(JournalError::AlreadyExists)
    ));
    assert!(std::fs::read(fixture.anchor()).unwrap() == original_anchor);
    assert!(
        std::fs::read(
            fixture
                .journal()
                .join(yolo_agent_control_journal::JOURNAL_FILE)
        )
        .unwrap()
            == original_receipts
    );
    owner.verify(&fixture.runtime).unwrap();
}
