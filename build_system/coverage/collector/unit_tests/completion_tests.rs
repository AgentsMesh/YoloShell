use crate::completion::Completion;
use crate::constants::{COMPLETION_MODE, INCOMPLETE_COVERAGE};
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

const MARKER_NAME: &str = "coverage.incomplete";
const PREEXISTING_FAILURE: &[u8] = b"earlier test stage failed";
const REPLACEMENT_NAME: &str = "retained-original";
const SECOND_LINK_NAME: &str = "linked-marker";
const CHANGED_MODE: u32 = 0o400;
const PERMISSION_BITS: u32 = 0o777;
const ALIASED_LINK_COUNT: u64 = 2;

#[test]
fn absent_completion_protocol_is_a_noop() {
    Completion::create(None).unwrap().finish().unwrap();
}

#[test]
fn only_successful_owned_collection_removes_its_marker() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(MARKER_NAME);
    let completion = Completion::create(Some(&path)).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), INCOMPLETE_COVERAGE);
    assert_eq!(
        std::fs::metadata(&path).unwrap().mode() & PERMISSION_BITS,
        COMPLETION_MODE
    );
    completion.finish().unwrap();
    assert!(!path.exists());
}

#[test]
fn aborted_collection_retains_the_failure_marker() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(MARKER_NAME);
    drop(Completion::create(Some(&path)).unwrap());
    assert_eq!(std::fs::read_to_string(path).unwrap(), INCOMPLETE_COVERAGE);
}

#[test]
fn preexisting_failure_markers_are_neither_overwritten_nor_removed() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(MARKER_NAME);
    std::fs::write(&path, PREEXISTING_FAILURE).unwrap();
    let before = std::fs::metadata(&path).unwrap();
    Completion::create(Some(&path)).unwrap().finish().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), PREEXISTING_FAILURE);
    assert_eq!(std::fs::metadata(&path).unwrap().ino(), before.ino());
}

#[test]
fn preexisting_symlinks_and_directories_remain_unmodified() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join(REPLACEMENT_NAME);
    std::fs::write(&target, PREEXISTING_FAILURE).unwrap();
    let path = directory.path().join(MARKER_NAME);
    symlink(&target, &path).unwrap();
    Completion::create(Some(&path)).unwrap().finish().unwrap();
    assert!(std::fs::symlink_metadata(&path).unwrap().is_symlink());
    assert_eq!(std::fs::read(&target).unwrap(), PREEXISTING_FAILURE);
    let nested = directory.path().join(SECOND_LINK_NAME);
    std::fs::create_dir(&nested).unwrap();
    Completion::create(Some(&nested)).unwrap().finish().unwrap();
    assert!(nested.is_dir());
}

#[test]
fn a_replaced_owned_marker_is_not_deleted() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(MARKER_NAME);
    let completion = Completion::create(Some(&path)).unwrap();
    std::fs::rename(&path, directory.path().join(REPLACEMENT_NAME)).unwrap();
    std::fs::write(&path, PREEXISTING_FAILURE).unwrap();
    assert!(completion.finish().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), PREEXISTING_FAILURE);
}

#[test]
fn aliased_owned_marker_cannot_be_cleared() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(MARKER_NAME);
    let completion = Completion::create(Some(&path)).unwrap();
    std::fs::hard_link(&path, directory.path().join(SECOND_LINK_NAME)).unwrap();
    assert!(completion.finish().is_err());
    assert_eq!(std::fs::metadata(path).unwrap().nlink(), ALIASED_LINK_COUNT);
}

#[test]
fn changed_content_and_permissions_are_rejected_without_removal() {
    for change_mode in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(MARKER_NAME);
        let completion = Completion::create(Some(&path)).unwrap();
        if change_mode {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(CHANGED_MODE)).unwrap();
        } else {
            std::fs::write(&path, PREEXISTING_FAILURE).unwrap();
        }
        assert!(completion.finish().is_err());
        assert!(path.exists());
    }
}

#[test]
fn missing_parent_and_missing_owned_marker_are_errors() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(MARKER_NAME);
    assert!(Completion::create(Some(&path.join(MARKER_NAME))).is_err());
    let completion = Completion::create(Some(&path)).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(completion.finish().is_err());
}

#[test]
fn a_symlink_to_the_retained_marker_cannot_authorize_completion() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(MARKER_NAME);
    let retained = directory.path().join(REPLACEMENT_NAME);
    let completion = Completion::create(Some(&path)).unwrap();
    std::fs::rename(&path, &retained).unwrap();
    symlink(&retained, &path).unwrap();
    assert!(completion.finish().is_err());
    assert!(std::fs::symlink_metadata(path).unwrap().is_symlink());
    assert_eq!(std::fs::read_to_string(retained).unwrap(), INCOMPLETE_COVERAGE);
}
