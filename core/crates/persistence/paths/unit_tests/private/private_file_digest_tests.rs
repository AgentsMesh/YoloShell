use super::super::*;
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt, PermissionsExt};

const TARGET_NAME: &str = "digest-state.bin";
const TEMPORARY_PREFIX: &str = ".yolo-digest-";
const TEMPORARY_SUFFIX: &str = ".tmp";
const TRUSTED_CONTENT: &[u8] = b"trusted-new";
const TAMPERED_CONTENT: &[u8] = b"tamper-now!";
const ORIGINAL_CONTENT: &[u8] = b"trusted-old";
const TAMPERED_ORIGINAL: &[u8] = b"tamper-old!";
const FILE_START_OFFSET: u64 = 0;
const SAVED_TARGET_NAME: &str = "saved-digest-state.bin";

#[test]
fn publish_rejects_post_rename_write_with_restored_mtime() {
    assert_eq!(TRUSTED_CONTENT.len(), TAMPERED_CONTENT.len());
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();
    let mut temporary = entry.temporary(TEMPORARY_PREFIX, TEMPORARY_SUFFIX).unwrap();
    temporary
        .file
        .write_all_at(TRUSTED_CONTENT, FILE_START_OFFSET)
        .unwrap();
    temporary.sync_and_inspect(&target, u64::MAX).unwrap();
    let metadata = temporary.file.metadata().unwrap();

    let result = entry.rename_temporary_with_hooks(
        &temporary,
        u64::MAX,
        || Ok(()),
        || {
            temporary
                .file
                .write_all_at(TAMPERED_CONTENT, FILE_START_OFFSET)
                .unwrap();
            restore_times(&temporary.file, &metadata);
        },
    );

    assert!(result.is_err());
    assert!(!target.exists());
    assert_eq!(
        fs::read(staging_path(root.path())).unwrap(),
        TAMPERED_CONTENT
    );
}

#[test]
fn publish_rechecks_public_path_after_digest_verification() {
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    let saved = root.path().join(SAVED_TARGET_NAME);
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();
    let mut temporary = entry.temporary(TEMPORARY_PREFIX, TEMPORARY_SUFFIX).unwrap();
    temporary
        .file
        .write_all_at(TRUSTED_CONTENT, FILE_START_OFFSET)
        .unwrap();
    temporary.sync_and_inspect(&target, u64::MAX).unwrap();
    entry.rename_temporary(&temporary, u64::MAX).unwrap();

    let result = entry.verify_installed_temporary_with_hook(&temporary, u64::MAX, || {
        fs::rename(&target, &saved)?;
        write_private(&target, TAMPERED_CONTENT)?;
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(target).unwrap(), TAMPERED_CONTENT);
    assert_eq!(fs::read(saved).unwrap(), TRUSTED_CONTENT);
}

#[test]
fn publish_rechecks_full_identity_after_digest_verification() {
    assert_eq!(TRUSTED_CONTENT.len(), TAMPERED_CONTENT.len());
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();
    let mut temporary = entry.temporary(TEMPORARY_PREFIX, TEMPORARY_SUFFIX).unwrap();
    temporary
        .file
        .write_all_at(TRUSTED_CONTENT, FILE_START_OFFSET)
        .unwrap();
    temporary.sync_and_inspect(&target, u64::MAX).unwrap();
    entry.rename_temporary(&temporary, u64::MAX).unwrap();
    let metadata = temporary.file.metadata().unwrap();

    let result = entry.verify_installed_temporary_with_hook(&temporary, u64::MAX, || {
        temporary
            .file
            .write_all_at(TAMPERED_CONTENT, FILE_START_OFFSET)?;
        restore_times(&temporary.file, &metadata);
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(target).unwrap(), TAMPERED_CONTENT);
}

#[test]
fn exchange_never_rolls_back_unproven_displaced_content() {
    assert_eq!(ORIGINAL_CONTENT.len(), TAMPERED_ORIGINAL.len());
    let root = private_root();
    let target = root.path().join(TARGET_NAME);
    write_private(&target, ORIGINAL_CONTENT).unwrap();
    let entry = private_file::PrivateFileEntry::ensure(&target).unwrap();
    let mut temporary = entry.temporary(TEMPORARY_PREFIX, TEMPORARY_SUFFIX).unwrap();
    temporary
        .file
        .write_all_at(TRUSTED_CONTENT, FILE_START_OFFSET)
        .unwrap();
    temporary.sync_and_inspect(&target, u64::MAX).unwrap();
    let previous = temporary
        .previous
        .as_ref()
        .unwrap()
        .file
        .try_clone()
        .unwrap();
    let metadata = previous.metadata().unwrap();

    let result = entry.rename_temporary_with_hooks(
        &temporary,
        u64::MAX,
        || Ok(()),
        || {
            previous
                .write_all_at(TAMPERED_ORIGINAL, FILE_START_OFFSET)
                .unwrap();
            restore_times(&previous, &metadata);
        },
    );

    assert!(result.is_err());
    assert_eq!(fs::read(&target).unwrap(), TRUSTED_CONTENT);
    assert_eq!(
        fs::read(staging_path(root.path())).unwrap(),
        TAMPERED_ORIGINAL
    );
}

fn restore_times(file: &fs::File, metadata: &fs::Metadata) {
    let times = [
        libc::timespec {
            tv_sec: metadata.atime(),
            tv_nsec: metadata.atime_nsec(),
        },
        libc::timespec {
            tv_sec: metadata.mtime(),
            tv_nsec: metadata.mtime_nsec(),
        },
    ];
    assert_eq!(
        unsafe { libc::futimens(file.as_raw_fd(), times.as_ptr()) },
        0
    );
}

fn private_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    create_private_dir(root.path()).unwrap();
    root
}

fn write_private(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn staging_path(directory: &Path) -> PathBuf {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap())
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(TEMPORARY_PREFIX)
        })
        .unwrap()
        .path()
}
