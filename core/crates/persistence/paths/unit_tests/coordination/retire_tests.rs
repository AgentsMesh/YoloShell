use super::*;
use crate::coordination::gc_tests::candidate_path;

const CANDIDATE_INDEX: usize = 109;
const OTHER_CANDIDATE_INDEX: usize = CANDIDATE_INDEX + 1;
const STATE_DIRECTORY: &str = "coordination";

fn root() -> (tempfile::TempDir, PrivateDirectory) {
    let temporary = tempfile::tempdir().unwrap();
    let root = PrivateDirectory::ensure(&temporary.path().join(STATE_DIRECTORY)).unwrap();
    (temporary, root)
}

fn lock(root: &PrivateDirectory, index: usize) -> PrivateFileLock {
    PrivateFileLock::try_acquire(
        &candidate_path(root.path(), index),
        super::super::MAX_COORDINATION_LOCK_BYTES,
    )
    .unwrap()
    .unwrap()
}

#[test]
fn lifecycle_contention_preserves_an_owned_lock_for_a_later_retry() {
    let (_temporary, root) = root();
    let mut lock = lock(&root, CANDIDATE_INDEX);
    let lifecycle = lifecycle::LifecycleLock::try_acquire(&root)
        .unwrap()
        .unwrap();

    assert!(!reclaim(&mut lock).unwrap());
    assert!(lock.path().exists());
    drop(lifecycle);
    assert!(reclaim(&mut lock).unwrap());
    assert!(!lock.path().exists());
}

#[test]
fn locked_reclaim_rejects_a_different_parent_descriptor() {
    let (_first_temporary, first) = root();
    let (_second_temporary, second) = root();
    let mut lock = lock(&first, OTHER_CANDIDATE_INDEX);

    assert!(reclaim_locked(&mut lock, &second).is_err());
    assert!(lock.path().exists());
    assert!(reclaim_locked(&mut lock, &first).unwrap());
}
