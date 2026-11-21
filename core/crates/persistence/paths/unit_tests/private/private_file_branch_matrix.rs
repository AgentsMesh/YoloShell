use super::*;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const TARGET_NAME: &str = "branch-matrix.bin";
const STAGE_PREFIX: &str = ".yolo-branch-matrix-";
const STAGE_SUFFIX: &str = ".tmp";
const ORIGINAL_BYTES: &[u8] = b"original-content";
const STAGED_BYTES: &[u8] = b"staged-content";
const REPLACEMENT_BYTES: &[u8] = b"replacement-content";
const NO_DIRECTORY_ENTRIES: usize = 0;

#[test]
fn bounded_cleanup_entry_point_enforces_its_entry_limit() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let name = format!("{STAGE_PREFIX}{}{STAGE_SUFFIX}", uuid::Uuid::nil());
    let residue = root.path().join(name);
    write_private(&residue, ORIGINAL_BYTES);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let result = entry.lock_and_cleanup_bounded(STAGE_PREFIX, STAGE_SUFFIX, NO_DIRECTORY_ENTRIES);
    assert!(result.is_err());
    assert!(residue.exists());
}

#[test]
fn restored_exchange_verifies_both_retained_content_versions() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    write_private(&target, ORIGINAL_BYTES);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let temporary = prepared_temporary(&entry, &target);
    let previous = temporary.previous.as_ref().unwrap().identity;

    entry
        .verify_previous_descriptor(&temporary, u64::MAX)
        .unwrap();
    entry
        .verify_temporary_descriptor(&temporary, u64::MAX)
        .unwrap();
    entry.verify_staging_name(&temporary, u64::MAX).unwrap();

    exchange_at(
        entry.parent_descriptor(),
        &temporary.name,
        entry.file_name(),
    )
    .unwrap();
    entry
        .verify_exchange(&temporary, previous, u64::MAX)
        .unwrap();
    exchange_at(
        entry.parent_descriptor(),
        &temporary.name,
        entry.file_name(),
    )
    .unwrap();

    entry
        .verify_restored_exchange(&temporary, u64::MAX)
        .unwrap();
}

#[test]
fn installed_verification_rejects_a_post_digest_replacement() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let temporary = prepared_temporary(&entry, &target);
    rename_exclusive_at(
        entry.parent_descriptor(),
        &temporary.name,
        entry.file_name(),
    )
    .unwrap();
    entry
        .verify_installed_temporary(&temporary, u64::MAX)
        .unwrap();
    let displaced = root.path().join("verified-before-replacement");

    let result = entry.verify_installed_temporary_with_hook(&temporary, u64::MAX, || {
        fs::rename(&target, &displaced)?;
        write_private(&target, REPLACEMENT_BYTES);
        Ok(())
    });

    assert!(result.is_err());
    assert!(entry.verify_staging_name(&temporary, u64::MAX).is_err());
    assert_eq!(fs::read(displaced).unwrap(), STAGED_BYTES);
    assert_eq!(fs::read(target).unwrap(), REPLACEMENT_BYTES);
}

#[test]
fn active_identity_and_retry_classification_cover_negative_states() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let temporary = prepared_temporary(&entry, &target);
    let active = temporary.name.clone();
    let identity = temporary.identity;
    entry.remove_temporary(&temporary).unwrap();
    assert!(!entry.active_identity_matches(&active, identity).unwrap());

    for kind in [
        std::io::ErrorKind::WouldBlock,
        std::io::ErrorKind::AlreadyExists,
    ] {
        assert!(private_file_temp_prepare::retryable(&io_error(kind)));
    }
    for kind in [
        std::io::ErrorKind::NotFound,
        std::io::ErrorKind::PermissionDenied,
    ] {
        assert!(!private_file_temp_prepare::retryable(&io_error(kind)));
    }
}

#[test]
fn staging_slot_selection_prefers_retired_then_quarantine() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let active =
        private_file_names::active_name(entry.file_name(), STAGE_PREFIX, STAGE_SUFFIX).unwrap();
    let staging = private_file_names::staging_name(entry.file_name(), STAGE_PREFIX).unwrap();
    let quarantine =
        private_file_names::quarantine_name(entry.file_name(), STAGE_PREFIX, &active).unwrap();
    let retired = private_file_names::retired_name(entry.file_name()).unwrap();
    assert_eq!(
        entry.available_staging_slot(&active, STAGE_PREFIX).unwrap(),
        staging
    );

    write_private(&named_path(&target, &quarantine), STAGED_BYTES);
    assert_eq!(
        entry.available_staging_slot(&active, STAGE_PREFIX).unwrap(),
        quarantine
    );
    write_private(&named_path(&target, &retired), ORIGINAL_BYTES);
    assert_eq!(
        entry.available_staging_slot(&active, STAGE_PREFIX).unwrap(),
        retired
    );
}

#[test]
fn staging_lock_contention_requests_a_retry_without_replacing_the_owner() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    write_private(&target, ORIGINAL_BYTES);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(target)
        .unwrap();
    let owner = private_file_temp_prepare::lock_or_retry(&file)
        .unwrap()
        .unwrap();

    assert!(
        private_file_temp_prepare::lock_or_retry(&file)
            .unwrap()
            .is_none()
    );
    drop(owner);
}

fn prepared_temporary(entry: &PrivateFileEntry, target: &Path) -> TemporaryPrivateFile {
    let mut temporary = entry.temporary(STAGE_PREFIX, STAGE_SUFFIX).unwrap();
    temporary.file.write_all(STAGED_BYTES).unwrap();
    temporary.sync_and_inspect(target, u64::MAX).unwrap();
    temporary
}

fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn named_path(target: &Path, name: &std::ffi::CStr) -> PathBuf {
    target.with_file_name(OsStr::from_bytes(name.to_bytes()))
}

fn io_error(kind: std::io::ErrorKind) -> anyhow::Error {
    std::io::Error::from(kind).into()
}
