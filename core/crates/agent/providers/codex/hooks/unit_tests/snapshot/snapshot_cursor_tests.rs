use super::snapshot_test_support::*;
use crate::codex_hooks::CodexSnapshotCursor;
use crate::codex_hooks::constants::{SNAPSHOT_FILE_NAME, test_values::UNKNOWN_SNAPSHOT_FIELD};

#[test]
fn missing_snapshot_persists_and_reloads_through_a_new_cursor() {
    let fixture = SnapshotFixture::new();
    let expected = snapshot();
    let expected_json = encoded(&expected);

    let (empty, loaded) = CodexSnapshotCursor::open(&fixture.path).unwrap();
    assert!(loaded.is_none());
    assert_eq!(empty.path(), fixture.path);
    assert!(!fixture.path.exists());

    let installed = empty.persist(&expected).unwrap();
    let (reloaded, actual) = CodexSnapshotCursor::open(&fixture.path).unwrap();
    assert_eq!(reloaded.path(), installed.path());
    assert_eq!(encoded(&actual.unwrap()), expected_json);
}

#[test]
fn stale_cursor_cannot_replace_a_newer_snapshot() {
    let fixture = SnapshotFixture::new();
    let (empty, loaded) = CodexSnapshotCursor::open(&fixture.path).unwrap();
    assert!(loaded.is_none());

    let current = empty.persist(&snapshot()).unwrap();
    assert!(empty.persist(&snapshot()).is_err());
    assert!(current.persist(&snapshot()).is_ok());
}

#[test]
fn malformed_and_unknown_snapshot_documents_are_rejected() {
    let malformed = SnapshotFixture::new();
    write_bytes(&malformed.path, MALFORMED_SNAPSHOT);
    assert!(CodexSnapshotCursor::open(&malformed.path).is_err());

    let unknown = SnapshotFixture::new();
    let mut value = encoded(&snapshot());
    value
        .as_object_mut()
        .unwrap()
        .insert(UNKNOWN_SNAPSHOT_FIELD.to_owned(), serde_json::Value::Null);
    write_json(&unknown.path, &value);
    assert!(CodexSnapshotCursor::open(&unknown.path).is_err());
}

#[test]
fn oversized_snapshot_is_rejected_before_deserialization() {
    let fixture = SnapshotFixture::new();
    let bytes = vec![OVERSIZED_FILL_BYTE; OVERSIZED_SNAPSHOT_BYTES];
    write_bytes(&fixture.path, &bytes);

    assert!(CodexSnapshotCursor::open(&fixture.path).is_err());
}

#[test]
fn cursor_accepts_only_an_absolute_canonical_snapshot_path() {
    let fixture = SnapshotFixture::new();
    assert!(CodexSnapshotCursor::open(SNAPSHOT_FILE_NAME).is_err());
    assert!(CodexSnapshotCursor::open(fixture.invocation.join(WRONG_FILE_NAME)).is_err());
}

#[test]
fn rooted_cursor_rejects_a_different_parent_descriptor() {
    let fixture = SnapshotFixture::new();
    let invocation =
        crate::paths::directory::PrivateDirectory::ensure(&fixture.invocation).unwrap();
    let foreign_path = fixture.root.path().join(FOREIGN_DIRECTORY);
    let foreign = crate::paths::directory::PrivateDirectory::ensure(&foreign_path).unwrap();

    assert!(CodexSnapshotCursor::open_in(&foreign, &fixture.path).is_err());
    assert!(CodexSnapshotCursor::open_in(&invocation, &fixture.path).is_ok());
}

#[test]
fn rooted_cursor_rejects_invocation_path_substitution() {
    let fixture = SnapshotFixture::new();
    let retained = crate::paths::directory::PrivateDirectory::ensure(&fixture.invocation).unwrap();
    let displaced = fixture.root.path().join(RETAINED_DIRECTORY);
    std::fs::rename(&fixture.invocation, &displaced).unwrap();
    crate::paths::directory::PrivateDirectory::ensure(&fixture.invocation).unwrap();

    assert!(CodexSnapshotCursor::open_in(&retained, &fixture.path).is_err());
}
