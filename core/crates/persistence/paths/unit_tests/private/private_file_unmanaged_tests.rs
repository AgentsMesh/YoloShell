use crate::paths::private_file::PrivateFileEntry;
use std::os::unix::fs::{PermissionsExt, symlink};

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const UNMANAGED_CONTENTS: &[u8] = b"unmanaged residue";

#[test]
fn unsafe_regular_file_is_unlinked_without_trusting_its_metadata() {
    let directory = private_directory();
    let path = directory.path().join("unsafe-file");
    std::fs::write(&path, UNMANAGED_CONTENTS).unwrap();
    let entry = PrivateFileEntry::open(&path).unwrap();

    assert!(entry.unlink_unmanaged_deferred().unwrap());
    entry.sync_parent().unwrap();
    assert!(!path.exists());
}

#[test]
fn unmanaged_symlink_is_unlinked_without_touching_its_target() {
    let directory = private_directory();
    let target = directory.path().join("target");
    let link = directory.path().join("link");
    std::fs::write(&target, UNMANAGED_CONTENTS).unwrap();
    symlink(&target, &link).unwrap();
    let entry = PrivateFileEntry::open(&link).unwrap();

    assert!(entry.unlink_unmanaged_deferred().unwrap());
    entry.sync_parent().unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), UNMANAGED_CONTENTS);
    assert!(std::fs::symlink_metadata(&link).is_err());
}

#[test]
fn unmanaged_directory_fails_closed_and_a_missing_entry_is_idempotent() {
    let directory = private_directory();
    let child = directory.path().join("child");
    std::fs::create_dir(&child).unwrap();
    let directory_entry = PrivateFileEntry::open(&child).unwrap();
    assert!(directory_entry.unlink_unmanaged_deferred().is_err());
    assert!(child.is_dir());

    let missing = PrivateFileEntry::open(&directory.path().join("missing")).unwrap();
    assert!(!missing.unlink_unmanaged_deferred().unwrap());
}

fn private_directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    std::fs::set_permissions(
        directory.path(),
        std::fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    directory
}
