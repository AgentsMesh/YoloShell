use super::*;
use crate::paths::{constants::PRIVATE_FILE_MODE, private_file::PrivateFileEntry};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const ORIGINAL_FILE: &str = "original.json";
const MOVED_FILE: &str = "moved.json";
const CONTENT: &[u8] = b"retained content";
const OTHER_CONTENT: &[u8] = b"different content";
const TEST_MAX_BYTES: u64 = 1024;
const UNSAFE_MODE: u32 = 0o644;

#[test]
fn durable_file_capture_reopens_as_fresh_strict_live_authority() {
    let directory = tempfile::tempdir().unwrap();
    crate::paths::directory::PrivateDirectory::ensure(directory.path()).unwrap();
    let path = directory.path().join(ORIGINAL_FILE);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .open(&path)
        .unwrap();
    file.write_all(CONTENT).unwrap();
    let (durable, live) =
        DurableFileVersion::capture_file_bounded(&file, &path, TEST_MAX_BYTES).unwrap();
    drop(file);
    let opened = PrivateFileEntry::open(&path)
        .unwrap()
        .open_read(TEST_MAX_BYTES)
        .unwrap();
    assert_eq!(
        opened.durable_version(&path, TEST_MAX_BYTES).unwrap(),
        durable
    );
    assert_eq!(
        durable
            .verify_file_bounded(opened.file(), &path, TEST_MAX_BYTES)
            .unwrap(),
        live
    );
    assert_eq!(durable.length(), CONTENT.len() as u64);
    assert_eq!(
        durable.object_identity().kind(),
        yolo_durable_identity::ObjectKind::RegularFile
    );
}

#[test]
fn same_object_rename_uses_content_comparison_and_copy_never_does() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(ORIGINAL_FILE);
    crate::paths::atomic_write_json(&path, &CONTENT).unwrap();
    let before = PrivateFileEntry::open(&path)
        .unwrap()
        .open_read(TEST_MAX_BYTES)
        .unwrap();
    let durable = before.durable_version(&path, TEST_MAX_BYTES).unwrap();
    let moved = directory.path().join(MOVED_FILE);
    std::fs::rename(&path, &moved).unwrap();
    let reopened = PrivateFileEntry::open(&moved)
        .unwrap()
        .open_read(TEST_MAX_BYTES)
        .unwrap();
    let after = reopened.durable_version(&moved, TEST_MAX_BYTES).unwrap();
    assert!(durable.same_content_version(after));
    assert_eq!(
        durable
            .verify_content_version_bounded(reopened.file(), &moved, TEST_MAX_BYTES)
            .unwrap(),
        reopened.identity()
    );
    assert_eq!(
        durable
            .verify_file_bounded(reopened.file(), &moved, TEST_MAX_BYTES)
            .is_ok(),
        durable.unchanged(after)
    );
    std::fs::copy(&moved, &path).unwrap();
    let copy = PrivateFileEntry::open(&path)
        .unwrap()
        .open_read(TEST_MAX_BYTES)
        .unwrap();
    assert!(
        durable
            .verify_content_version_bounded(copy.file(), &path, TEST_MAX_BYTES)
            .is_err()
    );
}

#[test]
fn changed_contents_or_unsafe_current_permissions_cannot_mint_live_cas() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(ORIGINAL_FILE);
    crate::paths::atomic_write_json(&path, &CONTENT).unwrap();
    let opened = PrivateFileEntry::open(&path)
        .unwrap()
        .open_read(TEST_MAX_BYTES)
        .unwrap();
    let durable = opened.durable_version(&path, TEST_MAX_BYTES).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(UNSAFE_MODE)).unwrap();
    assert!(
        durable
            .verify_file_bounded(opened.file(), &path, TEST_MAX_BYTES)
            .is_err()
    );
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    std::fs::write(&path, OTHER_CONTENT).unwrap();
    assert!(
        durable
            .verify_file_bounded(opened.file(), &path, TEST_MAX_BYTES)
            .is_err()
    );
    assert!(opened.durable_version(&path, TEST_MAX_BYTES).is_err());
    assert!(
        durable
            .verify_content_version_bounded(opened.file(), &path, TEST_MAX_BYTES)
            .is_err()
    );
    assert!(
        DurableFileVersion::capture_file_bounded(opened.file(), &path, u64::default()).is_err()
    );
}
