use super::super::*;
use std::fs;

#[path = "../../test_support/unit/private_file_swap_support.rs"]
mod support;
use support::*;

const TARGET_NAME: &str = "snapshot.bin";
const TEMPORARY_PREFIX: &str = ".yolo-snapshot-";
const TEMPORARY_SUFFIX: &str = ".tmp";
const ORIGINAL: &[u8] = b"durable-original";
const NEW: &[u8] = b"trusted-new";
const REPLACEMENT: &[u8] = b"boundary-replacement";

#[test]
fn publish_source_swap_rolls_back_without_losing_either_file() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    write_private(&target, ORIGINAL);
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();
    let _lock = entry
        .lock_and_cleanup(TEMPORARY_PREFIX, TEMPORARY_SUFFIX)
        .unwrap();
    let temporary = prepared_temporary(&entry, &target);
    let staging = staging_path(root.path());
    let trusted = root.path().join("trusted-saved");

    let result = entry.rename_temporary_with_hook(&temporary, u64::MAX, || {
        fs::rename(&staging, &trusted)?;
        write_private(&staging, REPLACEMENT);
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(&target).unwrap(), ORIGINAL);
    assert_eq!(fs::read(&staging).unwrap(), REPLACEMENT);
    assert_eq!(fs::read(&trusted).unwrap(), NEW);
    assert!(entry.remove_temporary(&temporary).is_err());
}

#[test]
fn publish_target_swap_preserves_old_target_and_replacement() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    write_private(&target, ORIGINAL);
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();
    let _lock = entry
        .lock_and_cleanup(TEMPORARY_PREFIX, TEMPORARY_SUFFIX)
        .unwrap();
    let temporary = prepared_temporary(&entry, &target);
    let staging = staging_path(root.path());
    let saved = root.path().join("old-target-saved");

    let result = entry.rename_temporary_with_hook(&temporary, u64::MAX, || {
        fs::rename(&target, &saved)?;
        write_private(&target, REPLACEMENT);
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(&target).unwrap(), REPLACEMENT);
    assert_eq!(fs::read(&saved).unwrap(), ORIGINAL);
    assert_eq!(fs::read(&staging).unwrap(), NEW);
}

#[test]
fn exclusive_publish_source_swap_restores_absent_target() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();
    let _lock = entry
        .lock_and_cleanup(TEMPORARY_PREFIX, TEMPORARY_SUFFIX)
        .unwrap();
    let temporary = prepared_temporary(&entry, &target);
    let staging = staging_path(root.path());
    let trusted = root.path().join("exclusive-trusted-saved");

    let result = entry.rename_temporary_with_hook(&temporary, u64::MAX, || {
        fs::rename(&staging, &trusted)?;
        write_private(&staging, REPLACEMENT);
        Ok(())
    });

    assert!(result.is_err());
    assert!(!target.exists());
    assert_eq!(fs::read(&staging).unwrap(), REPLACEMENT);
    assert_eq!(fs::read(&trusted).unwrap(), NEW);
}

#[test]
fn cleanup_boundary_swap_restores_replacement_without_unlinking_it() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    let residue = root.path().join(random_residue_name());
    let saved = root.path().join("stale-saved");
    write_private(&residue, ORIGINAL);
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();

    let result = entry.lock_and_cleanup_bounded_with(
        TEMPORARY_PREFIX,
        TEMPORARY_SUFFIX,
        MAX_PRIVATE_DIRECTORY_ENTRIES,
        |candidate| {
            fs::rename(candidate, &saved)?;
            write_private(candidate, REPLACEMENT);
            Ok(())
        },
    );

    assert!(result.is_err());
    assert_eq!(fs::read(&residue).unwrap(), REPLACEMENT);
    assert_eq!(fs::read(&saved).unwrap(), ORIGINAL);
    assert!(quarantine_paths(root.path()).is_empty());
}

#[test]
fn normal_cleanup_uses_one_truncated_quarantine_slot() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    let residue = root.path().join(random_residue_name());
    write_private(&residue, ORIGINAL);
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();

    drop(
        entry
            .lock_and_cleanup(TEMPORARY_PREFIX, TEMPORARY_SUFFIX)
            .unwrap(),
    );
    let quarantine = quarantine_paths(root.path());
    assert!(!residue.exists());
    assert_eq!(quarantine.len(), 1);
    assert_eq!(fs::metadata(&quarantine[0]).unwrap().len(), 0);
    drop(
        entry
            .lock_and_cleanup(TEMPORARY_PREFIX, TEMPORARY_SUFFIX)
            .unwrap(),
    );
    assert_eq!(quarantine_paths(root.path()), quarantine);
}

#[test]
fn deterministic_crash_quarantine_is_reused_by_the_next_writer() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();
    let temporary = prepared_temporary(&entry, &target);
    let active = staging_path(root.path());
    drop(temporary);

    drop(
        entry
            .lock_and_cleanup(TEMPORARY_PREFIX, TEMPORARY_SUFFIX)
            .unwrap(),
    );
    assert!(!active.exists());
    assert_eq!(quarantine_paths(root.path()).len(), 1);

    let reused = entry.temporary(TEMPORARY_PREFIX, TEMPORARY_SUFFIX).unwrap();
    assert!(active.exists());
    assert!(quarantine_paths(root.path()).is_empty());
    assert_eq!(reused.file.metadata().unwrap().len(), 0);
}

#[test]
fn staging_reuse_truncates_retained_descriptor_not_swapped_name() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();
    let first = prepared_temporary(&entry, &target);
    entry.remove_temporary(&first).unwrap();
    drop(first);
    let staging = staging_slot_path(root.path());
    fs::write(&staging, ORIGINAL).unwrap();
    let saved = root.path().join("retained-staging");
    let mut swap = true;

    let result = entry.temporary_with_hook(TEMPORARY_PREFIX, TEMPORARY_SUFFIX, || {
        if swap {
            swap = false;
            fs::rename(&staging, &saved)?;
            write_private(&staging, REPLACEMENT);
        }
        Ok(())
    });

    let temporary = result.unwrap();
    entry.remove_temporary(&temporary).unwrap();
    assert_eq!(fs::read(&saved).unwrap(), ORIGINAL);
    assert!(matching_paths(root.path(), TEMPORARY_PREFIX).is_empty());
}
