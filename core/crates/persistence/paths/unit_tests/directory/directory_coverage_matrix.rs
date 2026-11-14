use super::super::*;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const CREATED_NAME: &str = "created-directory";
const EXISTING_NAME: &str = "existing-directory";
const FILE_NAME: &str = "blocking-file";
const MISSING_NAME: &str = "missing-directory";
const CHILD_NAME: &str = "child";
const FILE_BYTES: &[u8] = b"not-a-directory";
const NUL_NAME_BYTES: &[u8] = b"nul\0name";
const INVALID_DESCRIPTOR: RawFd = -1;

#[test]
fn directory_primitives_classify_existing_created_and_non_directory_entries() {
    let root = tempfile::tempdir().unwrap();
    let parent = File::open(root.path()).unwrap();
    let existing_path = root.path().join(EXISTING_NAME);
    fs::create_dir(&existing_path).unwrap();
    let existing_name = entry_c_string(OsStr::new(EXISTING_NAME)).unwrap();
    let (_, created) = open_or_create(&parent, &existing_name, root.path()).unwrap();
    assert!(!created);

    let created_name = entry_c_string(OsStr::new(CREATED_NAME)).unwrap();
    let (_, created) = open_or_create(&parent, &created_name, root.path()).unwrap();
    assert!(created);

    let file_path = root.path().join(FILE_NAME);
    fs::write(&file_path, FILE_BYTES).unwrap();
    let file_name = entry_c_string(OsStr::new(FILE_NAME)).unwrap();
    assert!(open_or_create(&parent, &file_name, root.path()).is_err());
    assert!(open_directory(parent.as_raw_fd(), &file_name).is_err());
    assert!(open_directory(INVALID_DESCRIPTOR, &file_name).is_err());
}

#[test]
fn public_directory_entry_points_propagate_missing_and_blocked_components() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join(MISSING_NAME);
    assert!(PrivateDirectory::open(&missing).is_err());
    assert!(open_chain(&missing).is_err());

    let blocker = root.path().join(FILE_NAME);
    fs::write(&blocker, FILE_BYTES).unwrap();
    let blocked_child = blocker.join(CHILD_NAME);
    assert!(PrivateDirectory::open(&blocker).is_err());
    assert!(PrivateDirectory::ensure(&blocked_child).is_err());
    assert!(open_chain(&blocked_child).is_err());
}

#[test]
fn names_and_component_parsing_reject_embedded_nul_and_root() {
    let nul_name = OsStr::from_bytes(NUL_NAME_BYTES);
    assert!(entry_c_string(nul_name).is_err());
    assert!(
        normal_components(Path::new(std::path::MAIN_SEPARATOR_STR))
            .unwrap()
            .is_empty()
    );
    assert!(PrivateDirectory::ensure(Path::new(std::path::MAIN_SEPARATOR_STR)).is_err());
}
