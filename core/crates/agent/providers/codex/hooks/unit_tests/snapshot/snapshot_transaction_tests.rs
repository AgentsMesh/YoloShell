use super::snapshot_test_support::*;
use crate::codex_hooks::CodexSnapshotCursor;
use crate::codex_hooks::constants::{
    LOCK_RECORD_MAX_BYTES, SNAPSHOT_FILE_NAME, SNAPSHOT_LOCK_SUFFIX,
};
use crate::codex_hooks::snapshot_error::CodexSnapshotFenceError;
use std::path::{Path, PathBuf};

#[test]
fn target_replacement_between_load_and_persist_fails_closed() {
    let (fixture, cursor) = SnapshotFixture::populated();
    let original = std::fs::read(&fixture.path).unwrap();
    let foreign_path = fixture.foreign_path();
    write_json(&foreign_path, &snapshot());
    let foreign = std::fs::read(&foreign_path).unwrap();
    let displaced = fixture.invocation.join(DISPLACED_SNAPSHOT_FILE);
    std::fs::rename(&fixture.path, &displaced).unwrap();
    std::fs::rename(foreign_path, &fixture.path).unwrap();

    assert!(cursor.persist(&snapshot()).is_err());
    assert_eq!(std::fs::read(&fixture.path).unwrap(), foreign);
    assert_eq!(std::fs::read(displaced).unwrap(), original);
}

#[test]
fn target_disappearance_during_descriptor_read_is_rejected() {
    let (fixture, _cursor) = SnapshotFixture::populated();
    let path = fixture.path.clone();
    super::super::snapshot_storage::set_before_read_hook(move || {
        std::fs::remove_file(path).unwrap();
    });

    assert!(CodexSnapshotCursor::open(&fixture.path).is_err());
}

#[test]
fn target_replacement_after_open_is_rejected() {
    let (fixture, _cursor) = SnapshotFixture::populated();
    let foreign_path = fixture.foreign_path();
    write_json(&foreign_path, &snapshot());
    let path = fixture.path.clone();
    let displaced = fixture.invocation.join(DISPLACED_SNAPSHOT_FILE);
    super::super::snapshot_storage::set_after_open_hook(move || {
        std::fs::rename(&path, displaced).unwrap();
        std::fs::rename(foreign_path, path).unwrap();
    });

    assert!(CodexSnapshotCursor::open(&fixture.path).is_err());
}

#[test]
fn lock_replacement_after_acquire_is_rejected() {
    let (fixture, _cursor) = SnapshotFixture::populated();
    let lock = lock_path(&fixture.path);
    let displaced = fixture.invocation.join(DISPLACED_LOCK_FILE);
    let replacement = lock.clone();
    super::super::snapshot_lock::set_after_acquire(move || {
        std::fs::rename(lock, displaced).unwrap();
        let entry = crate::paths::private_file::PrivateFileEntry::ensure(&replacement).unwrap();
        drop(entry.open_rw_or_create(LOCK_RECORD_MAX_BYTES).unwrap());
    });

    assert!(CodexSnapshotCursor::open(&fixture.path).is_err());
}

#[test]
fn retained_directory_cursor_rejects_named_path_substitution() {
    let (fixture, cursor) = SnapshotFixture::populated();
    let original = std::fs::read(&fixture.path).unwrap();
    let retained = fixture.root.path().join(RETAINED_DIRECTORY);
    let foreign_path = fixture.foreign_path();
    write_json(&foreign_path, &snapshot());
    let foreign = std::fs::read(&foreign_path).unwrap();
    std::fs::rename(&fixture.invocation, &retained).unwrap();
    crate::paths::ensure_private_parent(&fixture.path).unwrap();
    std::fs::rename(foreign_path, &fixture.path).unwrap();

    assert!(cursor.persist(&snapshot()).is_err());
    assert_eq!(std::fs::read(&fixture.path).unwrap(), foreign);
    assert_eq!(
        std::fs::read(retained.join(SNAPSHOT_FILE_NAME)).unwrap(),
        original
    );
}

#[test]
fn publication_fence_reports_snapshot_changed() {
    let (fixture, cursor) = SnapshotFixture::populated();
    let proof = cursor.prove_current().unwrap();
    crate::codex_hooks::replace_snapshot_before_publication(&fixture.path);

    let error = proof.linearize_publication().unwrap_err();

    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::SnapshotChanged)
    ));
}

#[test]
fn publication_fence_reports_lock_path_changed() {
    let (fixture, cursor) = SnapshotFixture::populated();
    let proof = cursor.prove_current().unwrap();
    crate::codex_hooks::replace_snapshot_lock_before_publication(&fixture.path);

    let error = proof.linearize_publication().unwrap_err();

    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::LockPathChanged)
    ));
}

#[test]
fn publication_fence_reports_unlinked_and_overwritten_lock_paths() {
    let injections = [
        crate::codex_hooks::remove_snapshot_lock_before_publication as fn(&Path),
        crate::codex_hooks::overwrite_snapshot_lock_before_publication,
    ];
    for inject in injections {
        let (fixture, cursor) = SnapshotFixture::populated();
        let proof = cursor.prove_current().unwrap();
        inject(&fixture.path);

        let error = proof.linearize_publication().unwrap_err();

        assert!(matches!(
            error.downcast_ref::<CodexSnapshotFenceError>(),
            Some(CodexSnapshotFenceError::LockPathChanged)
        ));
    }
}

#[test]
fn proof_reports_directory_change_before_lock_acquisition() {
    let (fixture, cursor) = SnapshotFixture::populated();
    let retained = fixture.root.path().join(RETAINED_DIRECTORY);
    std::fs::rename(&fixture.invocation, retained).unwrap();

    let error = cursor.prove_current().err().unwrap();

    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::InvocationDirectoryChanged)
    ));
}

#[test]
fn publication_fence_reports_invocation_directory_changed() {
    let (fixture, cursor) = SnapshotFixture::populated();
    let proof = cursor.prove_current().unwrap();
    crate::codex_hooks::replace_snapshot_directory_before_publication(&fixture.path);

    let error = proof.linearize_publication().unwrap_err();

    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::InvocationDirectoryChanged)
    ));
}

#[cfg(unix)]
#[test]
fn snapshot_and_lock_symlinks_never_reach_external_targets() {
    assert_symlink_rejected(LinkKind::Snapshot);
    assert_symlink_rejected(LinkKind::Lock);
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum LinkKind {
    Snapshot,
    Lock,
}

#[cfg(unix)]
fn assert_symlink_rejected(kind: LinkKind) {
    let fixture = SnapshotFixture::new();
    crate::paths::ensure_private_parent(&fixture.path).unwrap();
    let outside = fixture.root.path().join(OUTSIDE_FILE);
    std::fs::write(&outside, MALFORMED_SNAPSHOT).unwrap();
    let link = match kind {
        LinkKind::Snapshot => fixture.path.clone(),
        LinkKind::Lock => lock_path(&fixture.path),
    };
    std::os::unix::fs::symlink(&outside, link).unwrap();

    assert!(CodexSnapshotCursor::open(&fixture.path).is_err());
    assert_eq!(std::fs::read(outside).unwrap(), MALFORMED_SNAPSHOT);
}

fn lock_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(SNAPSHOT_LOCK_SUFFIX);
    PathBuf::from(value)
}
