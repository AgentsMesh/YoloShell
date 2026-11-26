use super::super::constants::*;
use super::super::security;
use super::config_tests::write_mode;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

const GENERIC_LABEL: &str = "generic secure input";
const PRIVATE_BYTES: &[u8] = b"private";
const ROOT_OWNED_FILE: &str = "/etc/hosts";
const FIFO_REJECTION_TIMEOUT: Duration = Duration::from_secs(1);
const REPLACEMENT_BYTES: &[u8] = b"replace";
const GROWTH_BYTES: &[u8] = b"private-growth";
const UNSAFE_PRIVATE_FILE_MODE: u32 = 0o666;
const UNSAFE_PRIVATE_DIRECTORY_MODE: u32 = 0o755;
const ACL_PRIVATE_INPUT_FILE: &str = "private-input-with-acl";
const ACL_CHANGING_INPUT_FILE: &str = "changing-private-input-acl";
const ACL_TRUSTED_INPUT_FILE: &str = "trusted-input-with-acl";
const DENY_ACL_PRIVATE_INPUT_FILE: &str = "private-input-with-denial-acl";
const TRUSTED_FILE_MODE: u32 = 0o644;

#[derive(Debug)]
enum ReadMutation {
    Rewrite,
    Grow,
    Mode,
}

#[test]
fn generic_private_and_trusted_limits_use_secure_fallback() {
    let root = tempfile::tempdir().unwrap();
    let private = root.path().join("private-input");
    write_mode(&private, PRIVATE_BYTES, PRIVATE_FILE_MODE);
    assert_eq!(
        security::read_private_file(&private, GENERIC_LABEL).unwrap(),
        PRIVATE_BYTES
    );
    assert!(
        !security::read_trusted_file(Path::new(ROOT_OWNED_FILE), GENERIC_LABEL)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn private_directory_creation_and_existing_mode_are_descriptor_normalized() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    security::prepare_private_directory(&state).unwrap();
    security::prepare_private_directory(&state).unwrap();
    fs::set_permissions(
        &state,
        fs::Permissions::from_mode(UNSAFE_PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    security::prepare_private_directory(&state).unwrap();
    assert_eq!(
        fs::metadata(&state).unwrap().permissions().mode() & PERMISSION_BITS_MASK,
        PRIVATE_DIRECTORY_MODE
    );

    let file = root.path().join("not-a-directory");
    write_mode(&file, PRIVATE_BYTES, PRIVATE_FILE_MODE);
    assert!(security::prepare_private_directory(&file).is_err());
}

#[test]
fn scoped_paths_reject_root_parent_components_and_oversized_socket_names() {
    assert!(security::validate_absolute(Path::new("/"), GENERIC_LABEL).is_err());
    assert!(security::validate_absolute(Path::new("/private/../escape"), GENERIC_LABEL).is_err());
    let oversized = format!("/tmp/{}", "x".repeat(UNIX_SOCKET_PATH_MAX_BYTES));
    assert!(security::validate_socket_path(Path::new(&oversized)).is_err());
}

#[test]
fn private_file_reader_rejects_fifo_without_blocking() {
    use std::os::unix::ffi::OsStrExt;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("private-fifo");
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    let mode = libc::mode_t::try_from(PRIVATE_FILE_MODE).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), mode) }, 0);
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        sender
            .send(security::read_private_file(&path, GENERIC_LABEL).is_err())
            .unwrap();
    });
    assert!(receiver.recv_timeout(FIFO_REJECTION_TIMEOUT).unwrap());
}

#[test]
fn private_file_reader_rejects_hardlinks() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("private-original");
    let linked = root.path().join("private-linked");
    write_mode(&original, PRIVATE_BYTES, PRIVATE_FILE_MODE);
    fs::hard_link(&original, &linked).unwrap();
    assert!(security::read_private_file(&original, GENERIC_LABEL).is_err());
    assert!(security::read_private_file(&linked, GENERIC_LABEL).is_err());
}

#[test]
fn private_file_reader_revalidates_content_metadata_after_read() {
    for mutation in [
        ReadMutation::Rewrite,
        ReadMutation::Grow,
        ReadMutation::Mode,
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("changing-private-input");
        write_mode(&path, PRIVATE_BYTES, PRIVATE_FILE_MODE);
        let result = security::read_private_file_with_test_hook(&path, GENERIC_LABEL, || {
            match mutation {
                ReadMutation::Rewrite => fs::write(&path, REPLACEMENT_BYTES)?,
                ReadMutation::Grow => fs::write(&path, GROWTH_BYTES)?,
                ReadMutation::Mode => fs::set_permissions(
                    &path,
                    fs::Permissions::from_mode(UNSAFE_PRIVATE_FILE_MODE),
                )?,
            }
            Ok(())
        });
        assert!(
            result.is_err(),
            "mutation unexpectedly accepted: {mutation:?}"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn external_broker_inputs_reject_grant_acls_before_and_during_read() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ACL_PRIVATE_INPUT_FILE);
    write_mode(&path, PRIVATE_BYTES, PRIVATE_FILE_MODE);
    crate::acl_fixture::add_read_grant(&path);
    assert!(security::read_private_file(&path, GENERIC_LABEL).is_err());

    let changing = root.path().join(ACL_CHANGING_INPUT_FILE);
    write_mode(&changing, PRIVATE_BYTES, PRIVATE_FILE_MODE);
    let result = security::read_private_file_with_test_hook(&changing, GENERIC_LABEL, || {
        crate::acl_fixture::add_read_grant(&changing);
        Ok(())
    });
    assert!(result.is_err());

    let trusted = root.path().join(ACL_TRUSTED_INPUT_FILE);
    write_mode(&trusted, PRIVATE_BYTES, TRUSTED_FILE_MODE);
    crate::acl_fixture::add_read_grant(&trusted);
    assert!(security::read_trusted_file(&trusted, GENERIC_LABEL).is_err());

    let denied = root.path().join(DENY_ACL_PRIVATE_INPUT_FILE);
    write_mode(&denied, PRIVATE_BYTES, PRIVATE_FILE_MODE);
    crate::acl_fixture::add_delete_denial(&denied);
    assert_eq!(
        security::read_private_file(&denied, GENERIC_LABEL).unwrap(),
        PRIVATE_BYTES
    );
}

#[test]
fn private_file_reader_revalidates_published_path_after_read() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("replaced-private-input");
    let moved = root.path().join("moved-private-input");
    write_mode(&path, PRIVATE_BYTES, PRIVATE_FILE_MODE);
    let result = security::read_private_file_with_test_hook(&path, GENERIC_LABEL, || {
        fs::rename(&path, &moved)?;
        write_mode(&path, PRIVATE_BYTES, PRIVATE_FILE_MODE);
        Ok(())
    });
    assert!(result.is_err());
}
