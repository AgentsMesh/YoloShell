use super::*;
use std::fs::File;
use std::process::Command;
use tempfile::tempdir;

const CHMOD: &str = "/bin/chmod";
const ADD_ACL_ARGUMENT: &str = "+a";
const ALLOW_READ_ACL: &str = "everyone allow read";
const DENY_DELETE_ACL: &str = "everyone deny delete";

#[test]
fn empty_acl_is_accepted_and_added_acl_is_rejected() {
    let root = tempdir().unwrap();
    let path = root.path().join("state");
    let file = File::create(&path).unwrap();
    require_empty(&file, &path).unwrap();

    add_acl(&path, ALLOW_READ_ACL);
    assert!(require_empty(&file, &path).is_err());
    assert!(require_no_grants(&file, &path).is_err());

    clear(&file, &path).unwrap();
    require_empty(&file, &path).unwrap();
}

#[test]
fn ancestor_check_accepts_denials_but_private_check_does_not() {
    let root = tempdir().unwrap();
    let path = root.path().join("parent");
    std::fs::create_dir(&path).unwrap();
    let file = File::open(&path).unwrap();
    add_acl(&path, DENY_DELETE_ACL);

    require_no_grants(&file, &path).unwrap();
    assert!(require_empty(&file, &path).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn invalid_descriptor_acl_operations_fail_closed() {
    const INVALID_DESCRIPTOR: std::os::fd::RawFd = -1;
    assert!(platform::has_forbidden_entry(INVALID_DESCRIPTOR, true).is_err());
    assert!(platform::clear(INVALID_DESCRIPTOR).is_err());
}

fn add_acl(path: &std::path::Path, entry: &str) {
    let status = Command::new(CHMOD)
        .args([ADD_ACL_ARGUMENT, entry])
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
}
