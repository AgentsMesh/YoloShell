use std::path::Path;
use std::process::Command;

const CHMOD_EXECUTABLE: &str = "/bin/chmod";
const ADD_ACL_ARGUMENT: &str = "+a";
const GRANT_READ_ACL: &str = "everyone allow read";
const DENY_DELETE_ACL: &str = "everyone deny delete";

pub fn add_read_grant(path: &Path) {
    add(path, GRANT_READ_ACL);
}

pub fn add_delete_denial(path: &Path) {
    add(path, DENY_DELETE_ACL);
}

fn add(path: &Path, entry: &str) {
    let status = Command::new(CHMOD_EXECUTABLE)
        .args([ADD_ACL_ARGUMENT, entry])
        .arg(path)
        .status()
        .expect("run chmod to add an ACL fixture");
    assert!(status.success(), "chmod failed to add an ACL fixture");
}
