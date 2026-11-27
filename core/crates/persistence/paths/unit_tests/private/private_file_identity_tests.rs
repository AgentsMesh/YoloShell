use super::super::*;
use crate::paths::directory::PrivateDirectory;
use std::cell::Cell;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use tempfile::tempdir;

const CREATE_FAILURE: &str = "injected exclusive create publication failure";
const CREATE_FENCE_FAILURE: &str = "injected exclusive create fence failure";
const CREATE_MAX_BYTES: u64 = 64;
const CREATE_NAME: &str = "exclusive-create";
const CREATE_PAYLOAD: &[u8] = b"owned only after identity return";
const PERMISSION_BITS: u32 = 0o777;
const RECOVERABLE_CREATE_FAILURE: &str = "injected recoverable create interruption";

#[test]
fn recoverable_create_interruption_leaves_only_the_canonical_slot() {
    let root = tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let path = root.path().join(CREATE_NAME);
    let entry = private_file::PrivateFileEntry::open(&path).unwrap();
    let _hook =
        private_file::private_file_recoverable_create::install_recoverable_create_hook(|| {
            anyhow::bail!(RECOVERABLE_CREATE_FAILURE)
        });

    let result = entry.create_recoverable_bytes_exclusive(CREATE_PAYLOAD, CREATE_MAX_BYTES);

    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"");
    let metadata = std::fs::metadata(&path).unwrap();
    assert_eq!(
        metadata.permissions().mode() & PERMISSION_BITS,
        crate::paths::constants::PRIVATE_FILE_MODE
    );
    assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
    assert_eq!(
        metadata.nlink(),
        crate::paths::constants::EXPECTED_FILE_LINK_COUNT
    );
    assert_eq!(
        std::fs::read_dir(root.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>(),
        vec![path]
    );
}

#[test]
fn exclusive_create_rolls_back_a_publish_without_returned_identity() {
    let root = tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let path = root.path().join(CREATE_NAME);
    let entry = private_file::PrivateFileEntry::open(&path).unwrap();
    let _hook = private_file::install_exclusive_publish_hook(|| {
        crate::paths::directory::fail_next_sync();
        Err(anyhow::anyhow!(CREATE_FAILURE))
    });

    assert!(
        entry
            .create_bytes_exclusive(CREATE_PAYLOAD, CREATE_MAX_BYTES)
            .is_err()
    );
    assert_retryable(&entry, root.path(), &path);
}

#[test]
fn exclusive_create_cleans_an_unsynced_new_stage() {
    let root = tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let path = root.path().join(CREATE_NAME);
    let entry = private_file::PrivateFileEntry::open(&path).unwrap();
    crate::paths::directory::fail_next_sync();

    assert!(
        entry
            .create_bytes_exclusive(CREATE_PAYLOAD, CREATE_MAX_BYTES)
            .is_err()
    );
    assert_retryable(&entry, root.path(), &path);
}

#[test]
fn exclusive_create_cleans_failures_before_and_after_rename() {
    for parent_sync in [false, true] {
        let root = tempdir().unwrap();
        PrivateDirectory::ensure(root.path()).unwrap();
        let path = root.path().join(CREATE_NAME);
        let entry = private_file::PrivateFileEntry::open(&path).unwrap();
        let first_fence = Cell::new(true);

        let result =
            entry.create_bytes_exclusive_with_fence(CREATE_PAYLOAD, CREATE_MAX_BYTES, || {
                if first_fence.replace(false) && parent_sync {
                    crate::paths::directory::fail_next_sync();
                    Ok(())
                } else if parent_sync {
                    Ok(())
                } else {
                    anyhow::bail!(CREATE_FENCE_FAILURE)
                }
            });

        assert!(result.is_err());
        assert_retryable(&entry, root.path(), &path);
    }
}

#[test]
fn raw_file_identity_rejects_each_security_and_size_mismatch() {
    let root = tempdir().unwrap();
    let path = root.path().join("state");
    atomic_write_json(&path, &serde_json::json!({"ok": true})).unwrap();
    let original = raw_stat(&path);
    assert!(
        identity::FileIdentity::inspect_stat(&original, &path, original.st_size as u64).is_ok()
    );

    let mut non_regular = original;
    non_regular.st_mode = (non_regular.st_mode & !libc::S_IFMT) | libc::S_IFIFO;
    assert!(identity::FileIdentity::inspect_stat(&non_regular, &path, u64::MAX).is_err());

    let mut wrong_owner = original;
    wrong_owner.st_uid = unsafe { libc::geteuid() }.wrapping_add(1);
    assert!(identity::FileIdentity::inspect_stat(&wrong_owner, &path, u64::MAX).is_err());

    let mut linked = original;
    linked.st_nlink = linked.st_nlink.checked_add(1).unwrap();
    assert!(identity::FileIdentity::inspect_stat(&linked, &path, u64::MAX).is_err());

    let mut unsafe_mode = original;
    unsafe_mode.st_mode |= crate::paths::constants::GROUP_OR_OTHER_WRITE_MODE as libc::mode_t;
    assert!(identity::FileIdentity::inspect_stat(&unsafe_mode, &path, u64::MAX).is_err());

    let mut oversized = original;
    oversized.st_size = original.st_size.checked_add(1).unwrap();
    assert!(
        identity::FileIdentity::inspect_stat(&oversized, &path, original.st_size as u64).is_err()
    );

    let mut negative_size = original;
    negative_size.st_size = -1;
    assert!(identity::FileIdentity::inspect_stat(&negative_size, &path, u64::MAX).is_err());

    let mut negative_device = original;
    negative_device.st_dev = -1;
    assert!(identity::FileIdentity::inspect_stat(&negative_device, &path, u64::MAX).is_err());
}

fn raw_stat(path: &std::path::Path) -> libc::stat {
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    assert_eq!(unsafe { libc::stat(name.as_ptr(), stat.as_mut_ptr()) }, 0);
    unsafe { stat.assume_init() }
}

fn assert_retryable(
    entry: &private_file::PrivateFileEntry,
    directory: &std::path::Path,
    target: &std::path::Path,
) {
    assert_eq!(std::fs::read_dir(directory).unwrap().count(), 0);
    entry
        .create_bytes_exclusive(CREATE_PAYLOAD, CREATE_MAX_BYTES)
        .unwrap();
    assert_eq!(std::fs::read(target).unwrap(), CREATE_PAYLOAD);
}
