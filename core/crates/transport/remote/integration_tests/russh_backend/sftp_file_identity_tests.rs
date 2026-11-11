use super::*;
use std::path::Path;

const TEST_PATH: &str = "/allowed/file";
const TEST_SIZE: u64 = 8;
const TEST_UID: u32 = 501;
const TEST_GID: u32 = 20;
const TEST_MODE: u32 = 0o100600;
const TEST_MTIME: u32 = 1_700_000_000;
const TEST_DIRECTORY_MODE: u32 = 0o040600;
const TEST_SYMLINK_MODE: u32 = 0o120600;
const TEST_CHANGED_SIZE: u64 = 9;

#[test]
fn complete_regular_identity_is_stable() {
    let attributes = regular_attributes();
    let identity = SftpFileIdentity::regular(&attributes, Path::new(TEST_PATH)).unwrap();
    assert!(identity.verify(&attributes, Path::new(TEST_PATH)).is_ok());
}

#[test]
fn identity_rejects_missing_fields_non_files_and_drift() {
    let baseline = regular_attributes();
    let identity = SftpFileIdentity::regular(&baseline, Path::new(TEST_PATH)).unwrap();
    let mut missing_size = regular_attributes();
    missing_size.size = None;
    let mut missing_uid = regular_attributes();
    missing_uid.uid = None;
    let mut missing_gid = regular_attributes();
    missing_gid.gid = None;
    let mut missing_mode = regular_attributes();
    missing_mode.permissions = None;
    let mut missing_mtime = regular_attributes();
    missing_mtime.mtime = None;
    let mut directory = regular_attributes();
    directory.permissions = Some(TEST_DIRECTORY_MODE);
    let mut symlink = regular_attributes();
    symlink.permissions = Some(TEST_SYMLINK_MODE);
    let mut changed = regular_attributes();
    changed.size = Some(TEST_CHANGED_SIZE);
    for attributes in [
        missing_size,
        missing_uid,
        missing_gid,
        missing_mode,
        missing_mtime,
        directory,
        symlink,
        changed,
    ] {
        assert!(matches!(
            identity.verify(&attributes, Path::new(TEST_PATH)),
            Err(RemoteError::PathDenied(path)) if path == Path::new(TEST_PATH)
        ));
    }
}

fn regular_attributes() -> FileAttributes {
    FileAttributes {
        size: Some(TEST_SIZE),
        uid: Some(TEST_UID),
        gid: Some(TEST_GID),
        permissions: Some(TEST_MODE),
        mtime: Some(TEST_MTIME),
        ..Default::default()
    }
}
