use super::super::*;
use crate::paths::private_file_lock::PrivateFileLock;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
#[cfg(target_os = "macos")]
use std::process::Command;

const TARGET_NAME: &str = "unlink-proof-target";
const SAVED_NAME: &str = "unlink-proof-saved";
const ORIGINAL_BYTES: &[u8] = b"unlink-proof-original";
const REPLACEMENT_BYTES: &[u8] = b"unlink-proof-replacement";
const MAX_RECORD_BYTES: u64 = 64;
#[cfg(target_os = "macos")]
const CHMOD_PATH: &str = "/bin/chmod";
#[cfg(target_os = "macos")]
const ADD_ACL_ARGUMENT: &str = "+a";
#[cfg(target_os = "macos")]
const ALLOW_READ_ACL: &str = "everyone allow read";
#[cfg(target_os = "macos")]
const ACL_COMMAND_FAILED: &str = "failed to install adversarial ACL";

#[test]
fn exact_unlink_proves_retained_descriptor_postconditions() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let target = root.path().join(TARGET_NAME);
    let mut lock = PrivateFileLock::try_acquire(&target, MAX_RECORD_BYTES)
        .unwrap()
        .unwrap();
    lock.replace_contents(ORIGINAL_BYTES).unwrap();
    let isolation = PrivateFileEntry::open(&target)
        .unwrap()
        .unlink_isolation_entry()
        .unwrap()
        .path()
        .to_path_buf();

    lock.unlink_owned().unwrap();

    assert!(!target.exists());
    assert!(!isolation.exists());
}

#[test]
fn retained_descriptor_rejects_false_unlink_success() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let target = root.path().join(TARGET_NAME);
    let mut lock = PrivateFileLock::try_acquire(&target, MAX_RECORD_BYTES)
        .unwrap()
        .unwrap();
    lock.replace_contents(ORIGINAL_BYTES).unwrap();
    let entry = PrivateFileEntry::open(&target).unwrap();
    let isolation = entry.unlink_isolation_entry().unwrap().path().to_path_buf();
    let saved = root.path().join(SAVED_NAME);
    let isolation_for_hook = isolation.clone();
    let saved_for_hook = saved.clone();
    let _hook = entry.install_before_unlink_syscall_hook(move || {
        std::fs::rename(&isolation_for_hook, &saved_for_hook)?;
        write_private(&isolation_for_hook, REPLACEMENT_BYTES);
        Ok(())
    });

    let result = lock.unlink_owned();

    assert!(result.is_err());
    assert!(!target.exists());
    assert!(!isolation.exists());
    assert_eq!(std::fs::read(&saved).unwrap(), ORIGINAL_BYTES);
    assert_eq!(
        std::fs::metadata(saved).unwrap().nlink(),
        crate::paths::constants::EXPECTED_FILE_LINK_COUNT
    );
}

#[cfg(target_os = "macos")]
#[test]
fn retained_descriptor_rejects_acl_mutation_before_unlink() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let target = root.path().join(TARGET_NAME);
    let mut lock = PrivateFileLock::try_acquire(&target, MAX_RECORD_BYTES)
        .unwrap()
        .unwrap();
    lock.replace_contents(ORIGINAL_BYTES).unwrap();
    let entry = PrivateFileEntry::open(&target).unwrap();
    let isolation = entry.unlink_isolation_entry().unwrap().path().to_path_buf();
    let isolation_for_hook = isolation.clone();
    let _hook = entry.install_before_unlink_syscall_hook(move || {
        let status = Command::new(CHMOD_PATH)
            .args([ADD_ACL_ARGUMENT, ALLOW_READ_ACL])
            .arg(isolation_for_hook)
            .status()?;
        anyhow::ensure!(status.success(), ACL_COMMAND_FAILED);
        Ok(())
    });

    let result = lock.unlink_owned();

    assert!(result.is_err());
    assert!(!target.exists());
    assert!(!isolation.exists());
}

fn write_private(path: &std::path::Path, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_permissions(std::fs::Permissions::from_mode(
        crate::paths::constants::PRIVATE_FILE_MODE,
    ))
    .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
