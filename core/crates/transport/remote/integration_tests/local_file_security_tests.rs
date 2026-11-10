use super::super::constants::*;
use super::super::*;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;
use std::time::Duration;

const FIFO_RETURN_DEADLINE: Duration = Duration::from_secs(1);
const KEY_LINE: &str = "h ssh-ed25519 key\n";
const TEST_IDENTITY_TEXT: &str = "not a secret key";
const ACL_IDENTITY_FILE: &str = "identity-with-acl";
const ACL_KNOWN_HOSTS_FILE: &str = "known-hosts-with-acl";
const DENY_ACL_IDENTITY_FILE: &str = "identity-with-denial-acl";
const PRIVATE_INPUT_MODE: u32 = 0o600;
const READ_ONLY_PRIVATE_INPUT_MODE: u32 = 0o400;
const TRUSTED_INPUT_MODE: u32 = 0o644;

fn set_mode(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

fn make_fifo(path: &Path) {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    let mode = libc::mode_t::try_from(PRIVATE_INPUT_MODE).unwrap();
    // SAFETY: path is a valid NUL-terminated string and mode has no special bits.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), mode) }, 0);
}

fn known_hosts_with_bytes(bytes: usize) -> String {
    let full_lines = bytes / MAX_KNOWN_HOSTS_LINE_BYTES;
    let mut contents = String::with_capacity(bytes);
    for _ in 0..full_lines {
        contents.push('#');
        contents.push_str(&"x".repeat(MAX_KNOWN_HOSTS_LINE_BYTES - 2));
        contents.push('\n');
    }
    contents
}

#[test]
fn common_known_hosts_mode_is_accepted_but_unsafe_inputs_are_rejected() {
    let state = tempfile::tempdir().unwrap();
    let safe = state.path().join(TEST_KNOWN_HOSTS_FILE_NAME);
    std::fs::write(&safe, KEY_LINE).unwrap();
    set_mode(&safe, TRUSTED_INPUT_MODE);
    KnownHosts::from_path(&safe).unwrap();

    set_mode(&safe, 0o666);
    assert!(KnownHosts::from_path(&safe).is_err());
    let link = state.path().join("known_hosts-link");
    symlink(&safe, &link).unwrap();
    assert!(KnownHosts::from_path(&link).is_err());
    assert!(KnownHosts::from_path(state.path()).is_err());
}

#[test]
fn known_hosts_file_size_is_bounded_before_parsing() {
    let state = tempfile::tempdir().unwrap();
    let path = state.path().join(TEST_KNOWN_HOSTS_FILE_NAME);
    std::fs::write(&path, known_hosts_with_bytes(MAX_KNOWN_HOSTS_BYTES)).unwrap();
    set_mode(&path, TRUSTED_INPUT_MODE);
    KnownHosts::from_path(&path).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(MAX_KNOWN_HOSTS_BYTES as u64 + 1)
        .unwrap();
    assert!(KnownHosts::from_path(&path).is_err());
}

#[test]
fn private_identity_modes_are_accepted_and_unsafe_files_are_rejected() {
    let state = tempfile::tempdir().unwrap();
    let path = state.path().join("identity");
    std::fs::write(&path, TEST_IDENTITY_TEXT).unwrap();
    for mode in [PRIVATE_INPUT_MODE, READ_ONLY_PRIVATE_INPUT_MODE] {
        set_mode(&path, mode);
        assert!(super::super::local_file::read_identity_for_test(&path).is_ok());
    }
    set_mode(&path, TRUSTED_INPUT_MODE);
    assert!(super::super::local_file::read_identity_for_test(&path).is_err());

    let link = state.path().join("identity-link");
    symlink(&path, &link).unwrap();
    assert!(super::super::local_file::read_identity_for_test(&link).is_err());
    set_mode(&path, PRIVATE_INPUT_MODE);
    let hard_link = state.path().join("identity-hard-link");
    std::fs::hard_link(&path, &hard_link).unwrap();
    assert!(super::super::local_file::read_identity_for_test(&path).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn local_ssh_inputs_reject_grant_acls() {
    let state = tempfile::tempdir().unwrap();
    let identity = state.path().join(ACL_IDENTITY_FILE);
    std::fs::write(&identity, TEST_IDENTITY_TEXT).unwrap();
    set_mode(&identity, PRIVATE_INPUT_MODE);
    crate::acl_fixture::add_read_grant(&identity);
    assert!(super::super::local_file::read_identity_for_test(&identity).is_err());

    let known_hosts = state.path().join(ACL_KNOWN_HOSTS_FILE);
    std::fs::write(&known_hosts, KEY_LINE).unwrap();
    set_mode(&known_hosts, TRUSTED_INPUT_MODE);
    crate::acl_fixture::add_read_grant(&known_hosts);
    assert!(KnownHosts::from_path(&known_hosts).is_err());

    let denied = state.path().join(DENY_ACL_IDENTITY_FILE);
    std::fs::write(&denied, TEST_IDENTITY_TEXT).unwrap();
    set_mode(&denied, PRIVATE_INPUT_MODE);
    crate::acl_fixture::add_delete_denial(&denied);
    assert!(super::super::local_file::read_identity_for_test(&denied).is_ok());
}

#[test]
fn identity_file_size_accepts_max_and_rejects_max_plus_one() {
    let state = tempfile::tempdir().unwrap();
    let path = state.path().join("identity");
    std::fs::write(&path, "k").unwrap();
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(MAX_IDENTITY_FILE_BYTES as u64).unwrap();
    set_mode(&path, PRIVATE_INPUT_MODE);
    assert!(super::super::local_file::read_identity_for_test(&path).is_ok());
    file.set_len(MAX_IDENTITY_FILE_BYTES as u64 + 1).unwrap();
    assert!(super::super::local_file::read_identity_for_test(&path).is_err());
}

#[test]
fn fifo_inputs_return_promptly_without_a_writer() {
    let state = tempfile::tempdir().unwrap();
    let known = state.path().join("known-fifo");
    let identity = state.path().join("identity-fifo");
    make_fifo(&known);
    make_fifo(&identity);
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let known_result = KnownHosts::from_path(known).is_err();
        let identity_result = super::super::local_file::read_identity_for_test(&identity).is_err();
        sender.send((known_result, identity_result)).unwrap();
    });
    assert_eq!(
        receiver.recv_timeout(FIFO_RETURN_DEADLINE).unwrap(),
        (true, true)
    );
}
