use super::*;
use crate::paths::private_file_lock::PrivateFileLock;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const TARGET_NAME: &str = "unlink-recovery-target";
const MAX_BYTES: u64 = 64;
const ORIGINAL_BYTES: &[u8] = b"non-empty unlink recovery content";
const REPLACEMENT_BYTES: &[u8] = b"replacement survives recovery";
const INTERRUPTION_ERROR: &str = "simulated unlink syscall interruption";
const WOULD_BLOCK: std::io::ErrorKind = std::io::ErrorKind::WouldBlock;

#[test]
fn non_empty_canonical_isolation_is_reclaimed_before_recreation() {
    let root = tempfile::tempdir().unwrap();
    let (entry, isolation) = interrupted_entry(root.path());

    drop(entry.open_rw_or_create(MAX_BYTES).unwrap());

    assert!(entry.path().is_file());
    assert!(!isolation.exists());
}

#[test]
fn replacement_target_survives_canonical_isolation_reclaim() {
    let root = tempfile::tempdir().unwrap();
    let (entry, isolation) = interrupted_entry(root.path());
    write_private(entry.path(), REPLACEMENT_BYTES);

    drop(entry.open_rw_or_create(MAX_BYTES).unwrap());

    assert_eq!(fs::read(entry.path()).unwrap(), REPLACEMENT_BYTES);
    assert!(!isolation.exists());
}

#[test]
fn busy_canonical_isolation_is_deferred_without_data_loss() {
    let root = tempfile::tempdir().unwrap();
    let (entry, isolation) = interrupted_entry(root.path());
    let held = PrivateFileLock::try_acquire_existing(&isolation, MAX_BYTES)
        .unwrap()
        .unwrap();

    let error = entry.open_rw_or_create(MAX_BYTES).unwrap_err();

    assert_eq!(io_kind(&error), Some(WOULD_BLOCK));
    assert!(!entry.path().exists());
    assert_eq!(fs::read(&isolation).unwrap(), ORIGINAL_BYTES);
    drop(held);
    drop(entry.open_rw_or_create(MAX_BYTES).unwrap());
    assert!(!isolation.exists());
}

#[test]
fn durable_residue_enumeration_includes_canonical_unlink_isolation() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("private");
    PrivateDirectory::ensure(&directory).unwrap();
    let entry = PrivateFileEntry::from_directory(
        PrivateDirectory::open(&directory).unwrap(),
        &directory.join(TARGET_NAME),
    )
    .unwrap();
    let isolation = entry.unlink_isolation_entry().unwrap();
    let residues = entry.durable_write_residue_entries().unwrap();

    assert_eq!(
        residues.len(),
        PrivateFileEntry::DURABLE_WRITE_RESIDUE_ENTRY_COUNT
    );
    assert!(
        residues
            .iter()
            .any(|residue| { residue.file_name().to_bytes() == isolation.file_name().to_bytes() })
    );
}

fn interrupted_entry(root: &Path) -> (PrivateFileEntry, PathBuf) {
    let directory = root.join("private");
    PrivateDirectory::ensure(&directory).unwrap();
    let target = directory.join(TARGET_NAME);
    write_private(&target, ORIGINAL_BYTES);
    let entry = PrivateFileEntry::open(&target).unwrap();
    let opened = entry.open_read(MAX_BYTES).unwrap();
    let _hook = entry.install_before_unlink_syscall_hook(|| anyhow::bail!(INTERRUPTION_ERROR));
    assert!(
        entry
            .unlink_named_identity(entry.file_name(), opened.identity, MAX_BYTES, || Ok(()))
            .is_err()
    );
    drop(opened);
    let isolation = entry.unlink_isolation_entry().unwrap().path().to_path_buf();
    assert!(!target.exists());
    assert_eq!(fs::read(&isolation).unwrap(), ORIGINAL_BYTES);
    (entry, isolation)
}

fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(
        crate::paths::constants::PRIVATE_FILE_MODE,
    ))
    .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
