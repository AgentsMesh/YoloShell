use super::secure_vfs_alias_directory::open_verified;
use super::secure_vfs_alias_init::PublishedAlias;
use super::secure_vfs_alias_io::ALIAS_SEALED_MODE;
use super::secure_vfs_file::{ArtifactKind, SecureDescriptor};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;

const EMPTY_SQLITE_BYTES: &[u8] = b"";
const ACL_CATALOG_FILE: &str = "catalog-with-acl.sqlite";
const CATALOG_FILE: &str = "catalog.sqlite";

#[test]
fn writable_artifact_clears_acl_and_later_injection_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ACL_CATALOG_FILE);
    std::fs::write(&path, EMPTY_SQLITE_BYTES).unwrap();
    yolo_application_support_test_support::acl_fixture::add_read_grant(&path);

    let descriptor = descriptor(&path);
    descriptor.verify_current().unwrap();
    yolo_application_support_test_support::acl_fixture::add_read_grant(&path);
    assert!(descriptor.verify_current().is_err());
}

#[test]
fn alias_directory_rejects_acl_and_mode_transition_clears_it() {
    let root = tempfile::tempdir().unwrap();
    let artifact = root.path().join(CATALOG_FILE);
    std::fs::write(&artifact, EMPTY_SQLITE_BYTES).unwrap();
    let descriptor = descriptor(&artifact);
    let alias = PublishedAlias::create(&descriptor, ArtifactKind::Main).unwrap();
    yolo_application_support_test_support::acl_fixture::add_read_grant(alias.path());
    assert!(open_verified(alias.path()).is_err());

    alias.set_mode(ALIAS_SEALED_MODE).unwrap();
    let directory = open_verified(alias.path()).unwrap();
    crate::private_acl::require_empty(&directory, alias.path()).unwrap();
}

fn descriptor(path: &std::path::Path) -> SecureDescriptor {
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    let flags = rusqlite::ffi::SQLITE_OPEN_MAIN_DB | rusqlite::ffi::SQLITE_OPEN_READWRITE;
    SecureDescriptor::open(&name, flags).unwrap()
}
