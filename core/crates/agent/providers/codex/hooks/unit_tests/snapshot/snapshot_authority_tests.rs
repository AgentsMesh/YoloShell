use super::snapshot_test_support::*;
use crate::codex_hooks::CodexSnapshotCursor;
use crate::codex_hooks::constants::LOCK_RECORD_MAX_BYTES;
use crate::codex_hooks::snapshot_error::CodexSnapshotFenceError;

#[derive(Clone, Copy)]
enum CursorOperation {
    Persist,
    Prove,
    Claim,
}

#[derive(Clone, Copy)]
enum AuthoritySubstitution {
    Snapshot,
    Lock,
    Directory,
}

#[test]
fn missing_snapshot_cannot_mint_snapshot_lineage() {
    let fixture = SnapshotFixture::new();
    let (cursor, loaded) = CodexSnapshotCursor::open(&fixture.path).unwrap();
    assert!(loaded.is_none());

    assert_snapshot_changed(cursor.snapshot_lineage().unwrap_err());
}

#[test]
fn snapshot_lineage_tracks_each_durable_snapshot_identity() {
    let (fixture, cursor) = SnapshotFixture::populated();
    let initial = cursor.snapshot_lineage().unwrap();
    let (reopened, loaded) = CodexSnapshotCursor::open(&fixture.path).unwrap();
    assert!(loaded.is_some());
    reopened.require_lineage(initial).unwrap();

    let mut persisted = cursor.persist(&snapshot()).unwrap();
    let persisted_lineage = persisted.snapshot_lineage().unwrap();
    assert_ne!(persisted_lineage, initial);
    assert_snapshot_changed(persisted.require_lineage(initial).unwrap_err());

    persisted
        .claim(&snapshot())
        .unwrap()
        .publish_with(|proof| proof.linearize_publication())
        .unwrap();
    let claimed_lineage = persisted.snapshot_lineage().unwrap();
    assert_ne!(claimed_lineage, persisted_lineage);
    assert_snapshot_changed(persisted.require_lineage(persisted_lineage).unwrap_err());

    let (final_cursor, _) = CodexSnapshotCursor::open(&fixture.path).unwrap();
    final_cursor.require_lineage(claimed_lineage).unwrap();
}

#[test]
fn every_cursor_operation_rejects_a_recreated_lock() {
    for operation in [
        CursorOperation::Persist,
        CursorOperation::Prove,
        CursorOperation::Claim,
    ] {
        let (fixture, mut cursor) = SnapshotFixture::populated();
        replace_lock(&fixture);

        let error = match operation {
            CursorOperation::Persist => cursor.persist(&snapshot()).err().unwrap(),
            CursorOperation::Prove => cursor.prove_current().err().unwrap(),
            CursorOperation::Claim => cursor.claim(&snapshot()).err().unwrap(),
        };
        assert_lock_changed(error);
    }
}

#[test]
fn successor_open_requires_exact_predecessor_authority() {
    for substitution in [
        AuthoritySubstitution::Snapshot,
        AuthoritySubstitution::Lock,
        AuthoritySubstitution::Directory,
    ] {
        let (fixture, predecessor) = SnapshotFixture::populated();
        let lineage = predecessor.snapshot_lineage().unwrap();
        apply_substitution(&fixture, substitution);
        let (successor, loaded) = CodexSnapshotCursor::open(&fixture.path).unwrap();
        assert!(loaded.is_some());

        let error = successor.require_lineage(lineage).unwrap_err();

        assert!(match substitution {
            AuthoritySubstitution::Snapshot => is_snapshot_changed(&error),
            AuthoritySubstitution::Lock => is_lock_changed(&error),
            AuthoritySubstitution::Directory => matches!(
                error.downcast_ref::<CodexSnapshotFenceError>(),
                Some(CodexSnapshotFenceError::InvocationDirectoryChanged)
            ),
        });
    }
}

fn apply_substitution(fixture: &SnapshotFixture, substitution: AuthoritySubstitution) {
    match substitution {
        AuthoritySubstitution::Snapshot => {
            crate::codex_hooks::replace_snapshot_same_content(&fixture.path)
        }
        AuthoritySubstitution::Lock => replace_lock(fixture),
        AuthoritySubstitution::Directory => replace_directory(fixture),
    }
}

fn replace_lock(fixture: &SnapshotFixture) {
    let lock = super::super::snapshot_lock::lock_path(&fixture.path);
    let displaced = fixture.invocation.join(DISPLACED_LOCK_FILE);
    std::fs::rename(&lock, displaced).unwrap();
    let entry = crate::paths::private_file::PrivateFileEntry::ensure(&lock).unwrap();
    drop(entry.open_rw_or_create(LOCK_RECORD_MAX_BYTES).unwrap());
}

fn replace_directory(fixture: &SnapshotFixture) {
    let bytes = std::fs::read(&fixture.path).unwrap();
    let retained = fixture.root.path().join(RETAINED_DIRECTORY);
    std::fs::rename(&fixture.invocation, retained).unwrap();
    write_bytes(&fixture.path, &bytes);
}

fn assert_snapshot_changed(error: anyhow::Error) {
    assert!(is_snapshot_changed(&error));
}

fn assert_lock_changed(error: anyhow::Error) {
    assert!(is_lock_changed(&error));
}

fn is_snapshot_changed(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::SnapshotChanged)
    )
}

fn is_lock_changed(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::LockPathChanged)
    )
}
