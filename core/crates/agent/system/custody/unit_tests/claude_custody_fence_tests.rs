use super::fence_test_support as support;
use super::types::{CustodyPublication, CustodyState};
use super::*;

const ARCHIVED: bool = true;
const FIRST_RECORD: usize = 0;
const MANIFEST_TIMESTAMP_INCREMENT_MS: i64 = 1;
const NOT_ARCHIVED: bool = false;
const MANIFEST_AUTHORITY_ERROR_MISSING: &str = "manifest authority error is present";
use support::{Fixture, publish_intent_marker};

#[test]
fn managed_writer_persists_state_transitions() {
    let fixture = Fixture::new(NOT_ARCHIVED, None);

    arm(&fixture.ledger, &fixture.spec).unwrap();
    mark_containing(&fixture.ledger, &fixture.spec).unwrap();

    let ledger: CustodyLedger = crate::paths::read_json(&fixture.ledger).unwrap();
    assert_eq!(ledger.records[FIRST_RECORD].state, CustodyState::Containing);
}

#[test]
fn pending_intent_rejects_absent_ledger_creation() {
    let fixture = Fixture::new(NOT_ARCHIVED, None);
    publish_intent_marker(&fixture);

    assert!(arm(&fixture.ledger, &fixture.spec).is_err());
    assert!(!fixture.ledger.exists());
}

#[test]
fn intent_arriving_before_commit_rejects_absent_creator() {
    let fixture = Fixture::new(NOT_ARCHIVED, None);
    let intent = fixture.paths.reboot_migration_intent(fixture.terminal_id);
    persistence::set_before_write_hook(move || {
        crate::paths::atomic_write_json(&intent, &serde_json::Value::Null)
    });

    assert!(arm(&fixture.ledger, &fixture.spec).is_err());
    assert!(!fixture.ledger.exists());
}

#[test]
fn active_manifest_update_before_commit_allows_custody_creation() {
    let fixture = Fixture::new(NOT_ARCHIVED, None);
    let manifest = fixture.paths.manifest(fixture.terminal_id);
    persistence::set_before_write_hook(move || {
        let mut replacement: crate::model::TerminalManifest = crate::paths::read_json(&manifest)?;
        replacement.updated_at_ms = replacement
            .updated_at_ms
            .saturating_add(MANIFEST_TIMESTAMP_INCREMENT_MS);
        crate::paths::atomic_write_json(&manifest, &replacement)
    });

    arm(&fixture.ledger, &fixture.spec).unwrap();
    assert!(fixture.ledger.exists());
}

#[test]
fn manifest_archived_before_commit_rejects_absent_creator() {
    let fixture = Fixture::new(NOT_ARCHIVED, None);
    let manifest = fixture.paths.manifest(fixture.terminal_id);
    persistence::set_before_write_hook(move || {
        let mut replacement: crate::model::TerminalManifest = crate::paths::read_json(&manifest)?;
        replacement.archived = true;
        crate::paths::atomic_write_json(&manifest, &replacement)
    });

    assert!(arm(&fixture.ledger, &fixture.spec).is_err());
    assert!(!fixture.ledger.exists());
}

#[test]
fn manifest_replacement_after_commit_does_not_reclassify_committed_custody() {
    let fixture = Fixture::new(NOT_ARCHIVED, None);
    let manifest = fixture.paths.manifest(fixture.terminal_id);
    let original: crate::model::TerminalManifest = crate::paths::read_json(&manifest).unwrap();
    let expected_updated_at_ms = original
        .updated_at_ms
        .saturating_add(MANIFEST_TIMESTAMP_INCREMENT_MS);
    persistence::set_after_commit_hook(move || {
        let mut replacement: crate::model::TerminalManifest =
            crate::paths::read_json(&manifest).unwrap();
        replacement.updated_at_ms = replacement
            .updated_at_ms
            .saturating_add(MANIFEST_TIMESTAMP_INCREMENT_MS);
        crate::paths::atomic_write_json(&manifest, &replacement).unwrap();
    });

    arm(&fixture.ledger, &fixture.spec).unwrap();

    let replacement: crate::model::TerminalManifest =
        crate::paths::read_json(&fixture.paths.manifest(fixture.terminal_id)).unwrap();
    let ledger: CustodyLedger = crate::paths::read_json(&fixture.ledger).unwrap();
    let record = &ledger.records[FIRST_RECORD];
    assert_eq!(replacement.updated_at_ms, expected_updated_at_ms);
    assert_eq!(record.spec, fixture.spec);
    assert_eq!(record.state, CustodyState::Armed);
    assert_eq!(record.publication, CustodyPublication::Pending);
}

#[test]
fn exact_file_identity_rejects_a_stale_custody_writer() {
    let fixture = Fixture::new(NOT_ARCHIVED, None);
    arm(&fixture.ledger, &fixture.spec).unwrap();
    let path = fixture.ledger.clone();
    persistence::set_before_write_hook(move || {
        let mut replacement: CustodyLedger = crate::paths::read_json(&path)?;
        replacement.records[FIRST_RECORD].state = CustodyState::Indeterminate;
        crate::paths::atomic_write_json(&path, &replacement)
    });

    assert!(mark_containing(&fixture.ledger, &fixture.spec).is_err());
    let retained: CustodyLedger = crate::paths::read_json(&fixture.ledger).unwrap();
    assert_eq!(
        retained.records[FIRST_RECORD].state,
        CustodyState::Indeterminate
    );
}

#[test]
fn replaced_session_directory_rejects_the_writer() {
    let fixture = Fixture::new(NOT_ARCHIVED, None);
    let session = fixture.paths.session_dir(fixture.terminal_id);
    let displaced = fixture
        .paths
        .sessions_dir()
        .join(uuid::Uuid::new_v4().to_string());
    persistence::set_before_write_hook(move || {
        std::fs::rename(&session, &displaced)?;
        crate::paths::directory::PrivateDirectory::ensure(&session)?;
        Ok(())
    });

    assert!(arm(&fixture.ledger, &fixture.spec).is_err());
    assert!(!fixture.ledger.exists());
}

#[test]
fn archived_and_foreign_manifests_fail_closed() {
    let foreign = uuid::Uuid::new_v4();
    for fixture in [
        Fixture::new(ARCHIVED, None),
        Fixture::new(NOT_ARCHIVED, Some(foreign)),
    ] {
        let error = arm(&fixture.ledger, &fixture.spec).unwrap_err();
        assert!(error.is::<ManifestAuthorityUnavailable>());
        assert!(error.to_string().contains(context::MANIFEST_INVALID));
        assert!(!fixture.ledger.exists());
    }
}

#[test]
fn missing_manifest_has_typed_terminal_authority_loss() {
    let fixture = Fixture::new(NOT_ARCHIVED, None);
    std::fs::remove_file(fixture.paths.manifest(fixture.terminal_id)).unwrap();
    let settings = fixture
        .paths
        .session_dir(fixture.terminal_id)
        .join(crate::claude_settings_artifact::DIRECTORY_NAME);

    let error = begin_worker_termination(&settings, &fixture.spec.worker)
        .err()
        .expect(MANIFEST_AUTHORITY_ERROR_MISSING);

    assert!(error.is::<ManifestAuthorityUnavailable>());
    assert!(!fixture.ledger.exists());
}

#[test]
fn noncanonical_session_spelling_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let paths = crate::paths::Paths::new(directory.path().join(support::TEST_HOME_DIRECTORY));
    let terminal_id = uuid::Uuid::new_v4();
    let settings = paths
        .sessions_dir()
        .join(terminal_id.hyphenated().to_string().to_uppercase())
        .join(crate::claude_settings_artifact::DIRECTORY_NAME);

    assert!(path(&settings).is_err());
}

#[test]
fn settings_path_must_be_the_managed_claude_directory() {
    let fixture = Fixture::new(NOT_ARCHIVED, None);
    let foreign = fixture
        .paths
        .session_dir(fixture.terminal_id)
        .join(support::FOREIGN_SETTINGS_DIRECTORY);

    assert!(path(&foreign).is_err());
}
