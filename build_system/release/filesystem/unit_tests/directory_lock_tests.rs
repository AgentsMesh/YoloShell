use crate::{DirectoryHandle, constants::DIRECTORY_LOCK_ERROR};

#[test]
fn competing_publication_lock_fails_closed_without_waiting() {
    let root = tempfile::tempdir().unwrap();
    let owner = DirectoryHandle::open(root.path()).unwrap();
    let _lock = owner.lock_exclusive().unwrap();
    let contender = DirectoryHandle::open(root.path()).unwrap();
    let error = contender.lock_exclusive().unwrap_err();
    let report = format!("{error:#}");
    assert!(report.contains(DIRECTORY_LOCK_ERROR), "{report}");
}
