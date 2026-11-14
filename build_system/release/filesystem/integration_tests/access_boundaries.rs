use crate::{DirectoryHandle, TreeManifest, ensure_no_extended_acl};
use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;

const FILE_NAME: &str = "retained-evidence";
const BYTES: &[u8] = b"access failure must preserve release evidence";
const FILE_MODE: u32 = 0o400;
const DIRECTORY_MODE: u32 = 0o500;
const NO_ACCESS: u32 = 0;
const MODE_MASK: u32 = 0o777;
const INVALID_DESCRIPTOR: libc::c_int = -1;
const DESCRIPTION: &str = "release test evidence";
const CHMOD: &str = "/bin/chmod";
const ADD_ACL: &str = "+a";
const READ_ACL: &str = "everyone allow read";
const NESTED_CHILD: &str = "nested/entry";
const COMPONENT_ERROR: &str = "retained child name must be one path component";

#[test]
fn real_acl_and_invalid_descriptor_fail_closed_without_modifying_evidence() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(FILE_NAME);
    fs::write(&path, BYTES).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(FILE_MODE)).unwrap();
    let file = fs::File::open(&path).unwrap();
    ensure_no_extended_acl(file.as_raw_fd(), DESCRIPTION).unwrap();
    let status = std::process::Command::new(CHMOD)
        .args([ADD_ACL, READ_ACL])
        .arg(&path)
        .status()
        .unwrap();
    assert!(status.success());
    let acl_error = ensure_no_extended_acl(file.as_raw_fd(), DESCRIPTION).unwrap_err();
    assert!(acl_error.to_string().contains("must not have an ACL"));
    let descriptor_error = ensure_no_extended_acl(INVALID_DESCRIPTOR, DESCRIPTION).unwrap_err();
    assert!(format!("{descriptor_error:#}").contains("ACL"));
    assert_eq!(fs::read(&path).unwrap(), BYTES);
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & MODE_MASK,
        FILE_MODE
    );
}

#[test]
fn revoked_directory_access_rejects_lock_and_manifest_until_permissions_restore() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(FILE_NAME);
    fs::write(&path, BYTES).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(FILE_MODE)).unwrap();
    let directory = DirectoryHandle::open(root.path()).unwrap();
    directory.set_mode(DIRECTORY_MODE).unwrap();
    TreeManifest::capture(&directory).unwrap();
    directory.set_mode(NO_ACCESS).unwrap();
    let lock = directory.lock_exclusive();
    let manifest = TreeManifest::capture(&directory);
    directory.set_mode(DIRECTORY_MODE).unwrap();
    assert!(
        format!("{:#}", lock.unwrap_err()).contains(crate::constants::DIRECTORY_LOCK_OPEN_ERROR)
    );
    assert!(format!("{:#}", manifest.unwrap_err()).contains("duplicate manifest directory"));
    directory.lock_exclusive().unwrap();
    TreeManifest::capture(&directory).unwrap();
    assert_eq!(fs::read(&path).unwrap(), BYTES);
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & MODE_MASK,
        FILE_MODE
    );
    crate::restore_owner_write(root.path()).unwrap();
}

#[test]
fn invalid_child_components_never_escape_the_retained_parent() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join(FILE_NAME), BYTES).unwrap();
    fs::create_dir_all(root.path().join(NESTED_CHILD)).unwrap();
    DirectoryHandle::open(&root.path().join(NESTED_CHILD)).unwrap();
    let entries_before = fs::read_dir(root.path()).unwrap().count();
    let parent = DirectoryHandle::open(root.path()).unwrap();
    for name in [b"".as_slice(), NESTED_CHILD.as_bytes()] {
        let error = DirectoryHandle::open_at_bytes(&parent, name).unwrap_err();
        assert_eq!(error.to_string(), COMPONENT_ERROR);
    }
    assert!(DirectoryHandle::open_at_bytes(&parent, b"invalid\0entry").is_err());
    assert_eq!(fs::read(root.path().join(FILE_NAME)).unwrap(), BYTES);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), entries_before);
}
