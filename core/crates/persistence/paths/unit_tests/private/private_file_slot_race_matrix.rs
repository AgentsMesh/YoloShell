use super::super::*;
#[cfg(target_os = "macos")]
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const SOURCE_NAME: &str = "slot-source";
const TARGET_NAME: &str = "slot-target";
const SAVED_NAME: &str = "slot-saved";
const ORIGINAL_BYTES: &[u8] = b"slot-original";
const REPLACEMENT_BYTES: &[u8] = b"slot-replacement";
#[cfg(target_os = "macos")]
const SYSTEM_CALL_SUCCESS: libc::c_int = 0;
#[cfg(target_os = "macos")]
const CLEAR_FLAGS: libc::c_uint = 0;

#[test]
fn replacement_after_move_sync_is_rolled_back_without_losing_either_identity() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let source = root.path().join(SOURCE_NAME);
    let target = root.path().join(TARGET_NAME);
    let saved = root.path().join(SAVED_NAME);
    write_private(&source, ORIGINAL_BYTES);
    let source_entry = PrivateFileEntry::open(&source).unwrap();
    let opened = source_entry.open_read(u64::MAX).unwrap();
    let target_entry = PrivateFileEntry::ensure(&target).unwrap();
    let changed = target.clone();
    let retained = saved.clone();
    let _hook = target_entry.install_after_move_sync_hook(move || {
        fs::rename(&changed, &retained)?;
        write_private(&changed, REPLACEMENT_BYTES);
        Ok(())
    });

    let result = target_entry.move_named_identity(
        source_entry.file_name(),
        target_entry.file_name(),
        opened.identity,
        u64::MAX,
    );

    assert!(result.is_err());
    assert_eq!(fs::read(saved).unwrap(), ORIGINAL_BYTES);
    assert_eq!(fs::read(source).unwrap(), REPLACEMENT_BYTES);
}

#[cfg(target_os = "macos")]
#[test]
fn unlink_syscall_failure_preserves_the_verified_file() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let target = root.path().join(TARGET_NAME);
    write_private(&target, ORIGINAL_BYTES);
    let entry = PrivateFileEntry::open(&target).unwrap();
    let opened = entry.open_read(u64::MAX).unwrap();
    let isolation_name = private_file_names::unlink_isolation_name(entry.file_name()).unwrap();
    let isolation = root
        .path()
        .join(std::ffi::OsStr::from_bytes(isolation_name.to_bytes()));
    let immutable = isolation.clone();
    let _hook = entry.install_before_unlink_hook(move || {
        set_flags(&immutable, libc::UF_IMMUTABLE);
        Ok(())
    });

    let result =
        entry.unlink_named_identity(entry.file_name(), opened.identity, u64::MAX, || Ok(()));
    set_flags(&isolation, CLEAR_FLAGS);

    assert!(result.is_err());
    assert!(!target.exists());
    assert_eq!(fs::read(&isolation).unwrap(), ORIGINAL_BYTES);

    entry
        .unlink_named_identity(entry.file_name(), opened.identity, u64::MAX, || Ok(()))
        .unwrap();
    assert!(!target.exists());
    assert!(!isolation.exists());
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

#[cfg(target_os = "macos")]
fn set_flags(path: &Path, flags: libc::c_uint) {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::chflags(path.as_ptr(), flags) },
        SYSTEM_CALL_SUCCESS
    );
}
