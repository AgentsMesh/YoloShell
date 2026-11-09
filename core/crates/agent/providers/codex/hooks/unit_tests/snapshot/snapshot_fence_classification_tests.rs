use super::snapshot_test_support::*;
use crate::codex_hooks::CodexSnapshotCursor;
use crate::codex_hooks::constants::LOCK_RECORD_MAX_BYTES;
use crate::codex_hooks::snapshot_error::CodexSnapshotFenceError;

#[test]
fn authority_identity_capture_io_failure_is_indeterminate() {
    let (_fixture, cursor) = SnapshotFixture::populated();
    super::super::snapshot_lock::set_before_authority_capture(|| {
        Err(std::io::Error::from_raw_os_error(libc::EMFILE).into())
    });

    assert_indeterminate(cursor.prove_current().err().unwrap());
}

#[test]
fn directory_replacement_during_authority_capture_is_typed() {
    let (fixture, cursor) = SnapshotFixture::populated();
    let invocation = fixture.invocation.clone();
    let retained = fixture.root.path().join(RETAINED_DIRECTORY);
    let path = fixture.path.clone();
    super::super::snapshot_lock::set_before_authority_capture(move || {
        std::fs::rename(invocation, retained)?;
        crate::paths::ensure_private_parent(&path)?;
        Ok(())
    });

    assert_directory_changed(cursor.prove_current().err().unwrap());
}

#[test]
fn lock_replacement_during_failed_identity_capture_is_typed() {
    let (fixture, cursor) = SnapshotFixture::populated();
    let lock = super::super::snapshot_lock::lock_path(&fixture.path);
    let displaced = fixture.invocation.join(DISPLACED_LOCK_FILE);
    let replacement = lock.clone();
    super::super::snapshot_lock::set_before_authority_capture(move || {
        std::fs::rename(lock, displaced)?;
        let entry = crate::paths::private_file::PrivateFileEntry::ensure(&replacement)?;
        drop(entry.open_rw_or_create(LOCK_RECORD_MAX_BYTES)?);
        Err(std::io::Error::from_raw_os_error(libc::EIO).into())
    });

    assert_lock_changed(cursor.prove_current().err().unwrap());
}

#[test]
fn snapshot_replacement_during_load_is_typed() {
    let (fixture, _cursor) = SnapshotFixture::populated();
    let path = fixture.path.clone();
    super::super::snapshot_storage::set_before_read_hook(move || {
        crate::codex_hooks::replace_snapshot_same_content(&path)
    });

    assert_snapshot_changed(CodexSnapshotCursor::open(&fixture.path).err().unwrap());
}

#[test]
fn snapshot_replacement_during_claim_load_is_typed() {
    let (fixture, mut cursor) = SnapshotFixture::populated();
    let path = fixture.path.clone();
    super::super::snapshot_storage::set_before_read_hook(move || {
        crate::codex_hooks::replace_snapshot_same_content(&path)
    });

    assert_snapshot_changed(cursor.claim(&snapshot()).err().unwrap());
}

#[test]
fn snapshot_replacement_at_persist_commit_is_typed() {
    let (fixture, cursor) = SnapshotFixture::populated();
    let path = fixture.path.clone();
    super::super::snapshot_storage::set_before_persist_commit_hook(move || {
        crate::codex_hooks::replace_snapshot_same_content(&path)
    });

    assert_snapshot_changed(cursor.persist(&snapshot()).err().unwrap());
}

#[test]
fn persist_with_retained_fresh_target_after_write_error_is_indeterminate() {
    let (fixture, cursor) = SnapshotFixture::populated();
    let entry = crate::paths::private_file::PrivateFileEntry::open(&fixture.path).unwrap();
    let _failure = crate::paths::test_support::install_after_move_sync_hook(&entry, || {
        Err(anyhow::anyhow!(
            crate::codex_hooks::constants::errors::SNAPSHOT_CORRUPT
        ))
    });

    assert_indeterminate(cursor.persist(&snapshot()).err().unwrap());
    assert_snapshot_changed(cursor.prove_current().err().unwrap());
}

fn assert_snapshot_changed(error: anyhow::Error) {
    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::SnapshotChanged)
    ));
}

fn assert_lock_changed(error: anyhow::Error) {
    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::LockPathChanged)
    ));
}

fn assert_directory_changed(error: anyhow::Error) {
    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::InvocationDirectoryChanged)
    ));
}

fn assert_indeterminate(error: anyhow::Error) {
    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::Indeterminate(_))
    ));
}
