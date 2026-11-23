use super::super::*;
use crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES;
use std::ffi::{CString, OsStr};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const TARGET_NAME: &str = "cleanup-fault-target";
const STAGE_PREFIX: &str = ".yolo-cleanup-fault-";
const STAGE_SUFFIX: &str = ".tmp";
const SAVED_NAME: &str = "saved-cleanup-fault";
const RESIDUE_BYTES: &[u8] = b"cleanup-residue";
const REPLACEMENT_BYTES: &[u8] = b"cleanup-replacement";
const HOOK_ERROR: &str = "injected cleanup fence failure";
const NO_ENTRIES_ALLOWED: usize = 0;
const UNSAFE_FILE_MODE: u32 = 0o644;

#[derive(Clone, Copy, Debug)]
enum CleanupCase {
    EntryLimit,
    HookFailure,
    LockBusy,
    NoResidue,
    OccupiedQuarantine,
    SourceReplacement,
    Success,
    UnsafeResidue,
}

#[test]
fn cleanup_fault_matrix_preserves_unproven_files_and_recovers_healthy_residue() {
    for case in [
        CleanupCase::EntryLimit,
        CleanupCase::HookFailure,
        CleanupCase::LockBusy,
        CleanupCase::NoResidue,
        CleanupCase::OccupiedQuarantine,
        CleanupCase::SourceReplacement,
        CleanupCase::Success,
        CleanupCase::UnsafeResidue,
    ] {
        run_case(case);
    }
}

fn run_case(case: CleanupCase) {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let held = matches!(case, CleanupCase::LockBusy)
        .then(|| entry.lock_and_cleanup(STAGE_PREFIX, STAGE_SUFFIX).unwrap());
    let residue_name = format!("{STAGE_PREFIX}{}{STAGE_SUFFIX}", uuid::Uuid::nil());
    let residue = root.path().join(&residue_name);
    if !matches!(case, CleanupCase::NoResidue) {
        let mode = if matches!(case, CleanupCase::UnsafeResidue) {
            UNSAFE_FILE_MODE
        } else {
            PRIVATE_FILE_MODE
        };
        write_file(&residue, RESIDUE_BYTES, mode);
    }
    if matches!(case, CleanupCase::OccupiedQuarantine) {
        let source = CString::new(residue_name).unwrap();
        let name =
            private_file_names::quarantine_name(entry.file_name(), STAGE_PREFIX, &source).unwrap();
        write_file(
            &named_path(&target, &name),
            REPLACEMENT_BYTES,
            PRIVATE_FILE_MODE,
        );
    }
    let saved = root.path().join(SAVED_NAME);
    let max_entries = if matches!(case, CleanupCase::EntryLimit) {
        NO_ENTRIES_ALLOWED
    } else {
        MAX_PRIVATE_DIRECTORY_ENTRIES
    };
    let result =
        entry.lock_and_cleanup_bounded_with(STAGE_PREFIX, STAGE_SUFFIX, max_entries, |candidate| {
            match case {
                CleanupCase::HookFailure => anyhow::bail!(HOOK_ERROR),
                CleanupCase::SourceReplacement => {
                    fs::rename(candidate, &saved)?;
                    write_file(candidate, REPLACEMENT_BYTES, PRIVATE_FILE_MODE);
                    Ok(())
                }
                _ => Ok(()),
            }
        });
    let expected_success = matches!(case, CleanupCase::NoResidue | CleanupCase::Success);
    assert_eq!(result.is_ok(), expected_success, "{case:?}: {result:?}");
    if matches!(case, CleanupCase::Success) {
        assert!(!residue.exists());
    }
    drop(held);
}

fn write_file(path: &Path, bytes: &[u8], mode: u32) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(mode)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(mode))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn named_path(target: &Path, name: &std::ffi::CStr) -> PathBuf {
    target.with_file_name(OsStr::from_bytes(name.to_bytes()))
}
