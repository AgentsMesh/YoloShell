use super::*;
use crate::paths::constants::MAX_DURABLE_JSON_BYTES;
use std::cell::Cell;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

const TARGET_NAME: &str = "target-lock.json";
const OTHER_NAME: &str = "other-target.json";
const STAGE_PREFIX: &str = ".target-lock-stage-";
const STAGE_SUFFIX: &str = ".json";
const FENCE_FAILURE: &str = "injected target-lock commit fence failure";
const ORIGINAL_VALUE: u8 = 1;
const NEXT_VALUE: u8 = 2;

#[test]
fn authority_requires_the_matching_live_exclusive_target_lock() {
    let (root, entry, opened, lock) = locked_fixture();
    assert!(
        entry
            .open_retained_with_lock(MAX_DURABLE_JSON_BYTES)
            .is_err()
    );
    let other_path = root.path().join(OTHER_NAME);
    crate::paths::atomic_write_json(&other_path, &ORIGINAL_VALUE).unwrap();
    let other = PrivateFileEntry::open(&other_path).unwrap();
    let other_opened = other.open_retained(MAX_DURABLE_JSON_BYTES).unwrap();
    assert!(
        other
            .target_lock_authority(&other_opened, &lock, MAX_DURABLE_JSON_BYTES)
            .is_err()
    );
    entry
        .target_lock_authority(&opened, &lock, MAX_DURABLE_JSON_BYTES)
        .unwrap();
}

#[test]
fn typed_authority_installs_once_and_rejects_stale_reuse() {
    let (_root, entry, opened, lock) = locked_fixture();
    let authority = entry
        .target_lock_authority(&opened, &lock, MAX_DURABLE_JSON_BYTES)
        .unwrap();
    let installed =
        crate::paths::write_entry_expected_with_commit_fence_with_target_lock_authority(
            PrivateFileEntry::open(entry.path()).unwrap(),
            Some(opened.identity),
            &NEXT_VALUE,
            &authority,
            || Ok(()),
            || Ok(()),
        )
        .unwrap();
    assert!(!installed.same_object(opened.identity));
    assert!(
        crate::paths::write_entry_expected_with_commit_fence_with_target_lock_authority(
            PrivateFileEntry::open(entry.path()).unwrap(),
            Some(installed),
            &ORIGINAL_VALUE,
            &authority,
            || Ok(()),
            || Ok(()),
        )
        .is_err()
    );
}

#[test]
fn every_retained_staging_slot_locked_fails_closed() {
    let (root, entry) = fixture();
    let active =
        private_file_names::active_name(entry.file_name(), STAGE_PREFIX, STAGE_SUFFIX).unwrap();
    let candidates = [
        private_file_names::retired_name(entry.file_name()).unwrap(),
        private_file_names::quarantine_name(entry.file_name(), STAGE_PREFIX, &active).unwrap(),
        private_file_names::staging_name(entry.file_name(), STAGE_PREFIX).unwrap(),
    ];
    let candidate_count = candidates.len();
    let mut retained = Vec::new();
    for candidate in candidates {
        retained.push(locked_slot(&root, &candidate));
    }
    assert!(entry.available_staging_slot(&active, STAGE_PREFIX).is_err());
    assert_eq!(retained.len(), candidate_count);
}

#[test]
fn locked_recovery_then_canonical_slots_use_the_next_safe_slot() {
    let (root, entry) = fixture();
    let active =
        private_file_names::active_name(entry.file_name(), STAGE_PREFIX, STAGE_SUFFIX).unwrap();
    let retired = private_file_names::retired_name(entry.file_name()).unwrap();
    let staging = private_file_names::staging_name(entry.file_name(), STAGE_PREFIX).unwrap();
    let quarantine =
        private_file_names::quarantine_name(entry.file_name(), STAGE_PREFIX, &active).unwrap();
    let retired_file = locked_slot(&root, &retired);
    assert_eq!(
        entry.available_staging_slot(&active, STAGE_PREFIX).unwrap(),
        staging
    );
    let staging_file = locked_slot(&root, &staging);
    assert_eq!(
        entry.available_staging_slot(&active, STAGE_PREFIX).unwrap(),
        quarantine
    );
    drop((retired_file, staging_file));
}

#[test]
fn commit_fence_failure_never_notifies_the_commit_observer() {
    let (_root, entry, opened, lock) = locked_fixture();
    let authority = entry
        .target_lock_authority(&opened, &lock, MAX_DURABLE_JSON_BYTES)
        .unwrap();
    let observed = Cell::new(false);
    let result = crate::paths::write_entry_expected_with_commit_observer_with_target_lock_authority(
        PrivateFileEntry::open(entry.path()).unwrap(),
        Some(opened.identity),
        &NEXT_VALUE,
        &authority,
        || Ok(()),
        || anyhow::bail!(FENCE_FAILURE),
        |_| observed.set(true),
    );
    assert!(result.is_err());
    assert!(!observed.get());
    let current: u8 = crate::paths::read_json(entry.path()).unwrap();
    assert_eq!(current, ORIGINAL_VALUE);
}

fn locked_fixture() -> (
    tempfile::TempDir,
    PrivateFileEntry,
    OpenedPrivateFile,
    crate::paths::process_file_lock::ProcessFileLock,
) {
    let (root, entry) = fixture();
    crate::paths::atomic_write_json(entry.path(), &ORIGINAL_VALUE).unwrap();
    let (opened, lock) = entry
        .open_retained_with_lock(MAX_DURABLE_JSON_BYTES)
        .unwrap();
    (root, entry, opened, lock)
}

fn fixture() -> (tempfile::TempDir, PrivateFileEntry) {
    let root = tempfile::tempdir().unwrap();
    let entry = PrivateFileEntry::ensure(&root.path().join(TARGET_NAME)).unwrap();
    (root, entry)
}

fn write_private(path: &Path) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(b"stage").unwrap();
    file.sync_all().unwrap();
}

fn locked_slot(
    root: &tempfile::TempDir,
    name: &std::ffi::CString,
) -> (
    std::fs::File,
    crate::paths::process_file_lock::ProcessFileLock,
) {
    let path = root.path().join(name.to_string_lossy().as_ref());
    write_private(&path);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let lock = private_file_temp_prepare::lock_or_retry(&file)
        .unwrap()
        .unwrap();
    (file, lock)
}
