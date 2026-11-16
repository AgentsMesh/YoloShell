use super::super::KnownHosts;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const CHANGE_IDENTITY_FILE: &str = "change-identity";
const CHANGE_KNOWN_HOSTS_FILE: &str = "change-known-hosts";
const REPLACEMENT_FILE: &str = "replacement";
const PRIVATE_FILE_MODE: u32 = 0o600;
const PUBLIC_FILE_MODE: u32 = 0o644;
const ORIGINAL_BYTES: &[u8] = b"stable-input";
const SHORTER_LENGTH: u64 = 1;
const INVALID_UTF8: &[u8] = &[0xff];

#[test]
fn local_ssh_inputs_reject_invalid_utf8() {
    let root = tempfile::tempdir().unwrap();
    let identity = root.path().join(CHANGE_IDENTITY_FILE);
    write_mode(&identity, INVALID_UTF8, PRIVATE_FILE_MODE);
    assert!(super::super::local_file::read_identity_for_test(&identity).is_err());

    let known_hosts = root.path().join(CHANGE_KNOWN_HOSTS_FILE);
    write_mode(&known_hosts, INVALID_UTF8, PUBLIC_FILE_MODE);
    assert!(KnownHosts::from_path(known_hosts).is_err());
}

#[test]
fn identity_rejects_length_change_after_read() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(CHANGE_IDENTITY_FILE);
    write_mode(&path, ORIGINAL_BYTES, PRIVATE_FILE_MODE);
    let result = super::super::local_file::read_identity_with_test_hook(&path, |opened| {
        fs::OpenOptions::new()
            .write(true)
            .open(opened)?
            .set_len(SHORTER_LENGTH)
    });
    assert!(result.is_err());
}

#[test]
fn identity_rejects_path_replacement_after_read() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(CHANGE_IDENTITY_FILE);
    let replacement = root.path().join(REPLACEMENT_FILE);
    write_mode(&path, ORIGINAL_BYTES, PRIVATE_FILE_MODE);
    write_mode(&replacement, ORIGINAL_BYTES, PRIVATE_FILE_MODE);
    let result = super::super::local_file::read_identity_with_test_hook(&path, |_| {
        fs::rename(&replacement, &path)
    });
    assert!(result.is_err());
}

fn write_mode(path: &Path, contents: &[u8], mode: u32) {
    fs::write(path, contents).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}
