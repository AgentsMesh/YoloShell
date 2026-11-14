use std::path::{Path, PathBuf};

const RETAINED_DIRECTORY_EXTENSION: &str = "retained-fault";
const SNAPSHOT_DISPLACED_LOCK_FILE_NAME: &str = ".snapshot-lock-displaced";
const SNAPSHOT_HARD_LINK_FILE_NAME: &str = ".snapshot-hard-link";
const SNAPSHOT_REPLACEMENT_LOCK_FILE_NAME: &str = ".snapshot-lock-replacement";
const SNAPSHOT_REPLACEMENT_FILE_NAME: &str = ".snapshot-replacement";

#[derive(Clone, Copy)]
pub enum OutboxPostOperationFault {
    TargetRemoved,
    LockRemoved,
    DirectoryMoved,
}

#[derive(Clone, Copy)]
pub enum SnapshotClaimFault {
    TargetReplaced,
    TargetHardLinked,
    LockReplaced,
    DirectoryReplaced,
}

#[derive(Clone, Copy)]
pub enum SnapshotClaimStage {
    BeforeCommit,
    AfterRename,
}

pub fn inject_snapshot_claim_fault(
    path: &Path,
    stage: SnapshotClaimStage,
    fault: SnapshotClaimFault,
) {
    let path = path.to_path_buf();
    let inject = move || apply_snapshot_claim_fault(&path, fault);
    match stage {
        SnapshotClaimStage::BeforeCommit => {
            super::snapshot_claim::set_before_claim_commit_hook(inject);
        }
        SnapshotClaimStage::AfterRename => {
            super::snapshot_claim::set_after_claim_rename_hook(inject);
        }
    }
}

pub fn fail_snapshot_before_commit(path: &Path) {
    let lock = super::snapshot_lock::lock_path(path);
    super::snapshot_lock::set_after_acquire(move || remove_existing(&lock));
}

pub fn fail_snapshot_after_commit(path: &Path) {
    let lock = super::snapshot_lock::lock_path(path);
    super::snapshot_storage::set_after_persist_hook(move || remove_existing(&lock));
}

pub fn fail_next_outbox_sync_after_snapshot_publication(path: &Path) {
    let path = path.to_path_buf();
    super::outbox_storage::set_before_persist_commit_hook(path.clone(), move || {
        super::snapshot_cursor::set_before_publication_hook(move || {
            super::outbox_storage::set_before_persist_commit_hook(path, || {
                crate::paths::test_support::fail_next_directory_sync();
            });
        });
    });
}

pub fn replace_snapshot_after_commit(path: &Path) {
    let path = path.to_path_buf();
    super::snapshot_storage::set_after_persist_hook(move || replace_snapshot_same_content(&path));
}

pub fn replace_snapshot_before_publication(path: &Path) {
    let path = path.to_path_buf();
    super::snapshot_cursor::set_before_publication_hook(move || {
        replace_snapshot_same_content(&path)
    });
}

pub fn replace_snapshot_lock_before_publication(path: &Path) {
    let path = path.to_path_buf();
    super::snapshot_cursor::set_before_publication_hook(move || replace_snapshot_lock(&path));
}

pub fn remove_snapshot_lock_before_publication(path: &Path) {
    let lock = super::snapshot_lock::lock_path(path);
    super::snapshot_cursor::set_before_publication_hook(move || remove_existing(&lock));
}

pub fn overwrite_snapshot_lock_before_publication(path: &Path) {
    let path = path.to_path_buf();
    super::snapshot_cursor::set_before_publication_hook(move || overwrite_snapshot_lock(&path));
}

pub fn replace_snapshot_directory_before_publication(path: &Path) {
    let path = path.to_path_buf();
    super::snapshot_cursor::set_before_publication_hook(move || replace_snapshot_directory(&path));
}

pub fn replace_snapshot_same_content(path: &Path) {
    let bytes = std::fs::read(path).unwrap();
    let replacement = path.parent().unwrap().join(SNAPSHOT_REPLACEMENT_FILE_NAME);
    let entry = crate::paths::private_file::PrivateFileEntry::ensure(&replacement).unwrap();
    let max_bytes = u64::try_from(bytes.len()).unwrap();
    crate::paths::write_private_bytes_guarded(entry, &bytes, max_bytes, || Ok(())).unwrap();
    std::fs::rename(replacement, path).unwrap();
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

pub fn inject_outbox_fault(path: &Path, fault: OutboxPostOperationFault) {
    let path = path.to_path_buf();
    super::outbox_storage::set_after_operation_hook(move || match fault {
        OutboxPostOperationFault::TargetRemoved => remove_existing(&path),
        OutboxPostOperationFault::LockRemoved => {
            remove_existing(&super::lock::lock_path(&path));
        }
        OutboxPostOperationFault::DirectoryMoved => move_directory(&path),
    });
}

fn remove_existing(path: &Path) {
    std::fs::remove_file(path).unwrap();
}

fn replace_snapshot_lock(path: &Path) {
    let lock = super::snapshot_lock::lock_path(path);
    let displaced = path
        .parent()
        .unwrap()
        .join(SNAPSHOT_DISPLACED_LOCK_FILE_NAME);
    std::fs::rename(&lock, displaced).unwrap();
    let entry = crate::paths::private_file::PrivateFileEntry::ensure(&lock).unwrap();
    drop(
        entry
            .open_rw_or_create(super::constants::LOCK_RECORD_MAX_BYTES)
            .unwrap(),
    );
}

fn overwrite_snapshot_lock(path: &Path) {
    let lock = super::snapshot_lock::lock_path(path);
    let replacement = path
        .parent()
        .unwrap()
        .join(SNAPSHOT_REPLACEMENT_LOCK_FILE_NAME);
    let entry = crate::paths::private_file::PrivateFileEntry::ensure(&replacement).unwrap();
    drop(
        entry
            .open_rw_or_create(super::constants::LOCK_RECORD_MAX_BYTES)
            .unwrap(),
    );
    std::fs::rename(replacement, lock).unwrap();
}

fn replace_snapshot_directory(path: &Path) {
    let bytes = std::fs::read(path).unwrap();
    move_directory(path);
    let entry = crate::paths::private_file::PrivateFileEntry::ensure(path).unwrap();
    let max_bytes = u64::try_from(bytes.len()).unwrap();
    crate::paths::write_private_bytes_guarded(entry, &bytes, max_bytes, || Ok(())).unwrap();
}

fn apply_snapshot_claim_fault(path: &Path, fault: SnapshotClaimFault) {
    match fault {
        SnapshotClaimFault::TargetReplaced => replace_snapshot_same_content(path),
        SnapshotClaimFault::TargetHardLinked => {
            let link = path.parent().unwrap().join(SNAPSHOT_HARD_LINK_FILE_NAME);
            std::fs::hard_link(path, link).unwrap();
        }
        SnapshotClaimFault::LockReplaced => replace_snapshot_lock(path),
        SnapshotClaimFault::DirectoryReplaced => replace_snapshot_directory(path),
    }
}

fn move_directory(path: &Path) {
    let directory = path.parent().unwrap();
    let retained = retained_path(directory);
    std::fs::rename(directory, retained).unwrap();
}

fn retained_path(directory: &Path) -> PathBuf {
    directory.with_extension(RETAINED_DIRECTORY_EXTENSION)
}
