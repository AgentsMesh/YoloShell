use std::ffi::CString;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use yolo_release_archive_audit::{audit_archive_path, audit_bytes, audit_opened_archive};
use yolo_release_archive_audit_test_support::valid_archive;
use yolo_release_contracts::{ARCHIVE_NAME, PRIVATE_FILE_MODE};

#[cfg(target_os = "macos")]
mod text_attribute_extraction;

const ALIAS_NAME: &str = "alias.zip";
const INVALID_BYTES: &[u8] = b"not a ZIP archive";
const SUCCESS: libc::c_int = 0;

#[test]
fn named_archive_and_open_descriptor_produce_the_same_complete_inventory() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ARCHIVE_NAME);
    let fixture = valid_archive();
    fs::write(&path, &fixture.bytes).unwrap();
    let expected = audit_bytes(&fixture.bytes).unwrap();
    let named = audit_archive_path(&path).unwrap();
    let mut file = File::open(&path).unwrap();
    let end = file.seek(SeekFrom::End(0)).unwrap();
    let opened = audit_opened_archive(&file).unwrap();
    assert_eq!(named.entries, expected.entries);
    assert_eq!(opened.entries, expected.entries);
    assert_eq!(file.stream_position().unwrap(), end);
}

#[test]
fn missing_files_directories_and_symbolic_links_are_not_archive_inputs() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ARCHIVE_NAME);
    assert!(audit_archive_path(&path).is_err());
    assert!(audit_archive_path(root.path()).is_err());
    let fixture = valid_archive();
    fs::write(&path, &fixture.bytes).unwrap();
    let alias = root.path().join(ALIAS_NAME);
    symlink(&path, &alias).unwrap();
    assert!(audit_archive_path(&alias).is_err());
    assert!(audit_archive_path(&path).is_ok());
}

#[test]
fn fifo_archive_is_rejected_without_waiting_for_a_writer() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ARCHIVE_NAME);
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::mkfifo(name.as_ptr(), PRIVATE_FILE_MODE as libc::mode_t) },
        SUCCESS
    );
    assert!(audit_archive_path(&path).is_err());
}

#[test]
fn malformed_named_archives_fail_without_rewriting_the_input() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ARCHIVE_NAME);
    for bytes in [INVALID_BYTES, &[]] {
        fs::write(&path, bytes).unwrap();
        assert!(audit_archive_path(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn write_only_archive_descriptors_cannot_pass_binary_audit() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ARCHIVE_NAME);
    fs::write(&path, valid_archive().bytes).unwrap();
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    assert!(audit_opened_archive(&file).is_err());
}
