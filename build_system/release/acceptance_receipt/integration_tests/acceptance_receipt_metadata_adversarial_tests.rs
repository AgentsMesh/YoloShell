use super::acceptance_receipt_support::{bytes, expectation, open};
use super::test_constants::PRIVATE_FILE_MODE;
use super::test_constants::{
    ACL_ADD_ARGUMENT, ACL_REMOVE_ARGUMENT, CHMOD_EXECUTABLE, TEST_DENY_DELETE_ACL,
};
use crate::constants::READ_ONLY_FILE_MODE;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use tempfile::tempdir;

const ROOT_OWNER_UID: libc::uid_t = libc::uid_t::MIN;
const ALTERNATE_OWNER_UID: libc::uid_t = ROOT_OWNER_UID.saturating_add(1);
const PRESERVE_GROUP_ID: libc::gid_t = libc::gid_t::MAX;
const RECEIPT_FILE_NAME: &str = "receipt.json";
const RECEIPT_LINK_NAME: &str = "receipt-link.json";

fn write_receipt(path: &std::path::Path) {
    fs::write(path, bytes()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(READ_ONLY_FILE_MODE)).unwrap();
}

#[test]
fn mode_change_is_rejected_after_open() {
    let directory = tempdir().unwrap();
    let path = directory.path().join(RECEIPT_FILE_NAME);
    write_receipt(&path);
    let receipt = open(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    assert!(receipt.verify(&expectation()).is_err());
}

#[test]
fn hard_link_change_is_rejected_after_open() {
    let directory = tempdir().unwrap();
    let path = directory.path().join(RECEIPT_FILE_NAME);
    write_receipt(&path);
    let receipt = open(&path).unwrap();
    let link = directory.path().join(RECEIPT_LINK_NAME);
    fs::hard_link(&path, &link).unwrap();
    assert!(receipt.verify(&expectation()).is_err());
}

#[test]
fn owner_change_is_rejected_after_open_when_authorized() {
    let directory = tempdir().unwrap();
    let path = directory.path().join(RECEIPT_FILE_NAME);
    write_receipt(&path);
    let receipt = open(&path).unwrap();
    let current_uid = unsafe { libc::geteuid() };
    let target_uid = if current_uid == ROOT_OWNER_UID {
        ALTERNATE_OWNER_UID
    } else {
        ROOT_OWNER_UID
    };
    let path_c = CString::new(path.as_os_str().as_bytes()).unwrap();
    let result = unsafe { libc::chown(path_c.as_ptr(), target_uid, PRESERVE_GROUP_ID) };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        assert!(matches!(
            error.raw_os_error(),
            Some(libc::EPERM | libc::EACCES)
        ));
        return;
    }
    assert!(receipt.verify(&expectation()).is_err());
}

#[test]
fn extended_acl_change_is_rejected_after_open() {
    let directory = tempdir().unwrap();
    let path = directory.path().join(RECEIPT_FILE_NAME);
    write_receipt(&path);
    let receipt = open(&path).unwrap();
    run_chmod(&[ACL_ADD_ARGUMENT, TEST_DENY_DELETE_ACL], &path);
    let result = receipt.verify(&expectation());
    run_chmod(&[ACL_REMOVE_ARGUMENT], &path);
    assert!(result.is_err());
}

#[test]
fn hard_link_is_rejected_when_the_receipt_is_first_opened() {
    let directory = tempdir().unwrap();
    let path = directory.path().join(RECEIPT_FILE_NAME);
    write_receipt(&path);
    fs::hard_link(&path, directory.path().join(RECEIPT_LINK_NAME)).unwrap();
    assert_eq!(
        open(&path).err().unwrap().to_string(),
        crate::constants::RECEIPT_FILE_LINK_ERROR
    );
}

#[test]
fn directory_receipt_is_rejected_before_content_parsing() {
    let directory = tempdir().unwrap();
    assert_eq!(
        open(directory.path()).err().unwrap().to_string(),
        crate::constants::RECEIPT_FILE_TYPE_ERROR
    );
}

#[test]
fn canonical_digest_revalidates_the_retained_receipt_file() {
    let directory = tempdir().unwrap();
    let path = directory.path().join(RECEIPT_FILE_NAME);
    write_receipt(&path);
    let receipt = open(&path).unwrap();
    receipt.canonical_digest().unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    assert!(receipt.canonical_digest().is_err());
}

#[test]
fn public_file_reader_never_accepts_the_test_signing_authority() {
    let directory = tempdir().unwrap();
    let path = directory.path().join(RECEIPT_FILE_NAME);
    write_receipt(&path);
    assert_eq!(
        crate::AcceptanceReceipt::open(&path)
            .err()
            .unwrap()
            .to_string(),
        crate::constants::RECEIPT_SIGNATURE_ERROR
    );
}

fn run_chmod(arguments: &[&str], path: &std::path::Path) {
    let result = Command::new(CHMOD_EXECUTABLE)
        .args(arguments)
        .arg(path)
        .status()
        .unwrap();
    assert!(result.success());
}
