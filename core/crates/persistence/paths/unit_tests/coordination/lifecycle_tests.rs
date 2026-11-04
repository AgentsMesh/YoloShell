use super::*;

const STATE_DIRECTORY: &str = "coordination";

#[test]
fn file_lock_contention_releases_the_mach_lease_for_retry() {
    let temporary = tempfile::tempdir().unwrap();
    let root = PrivateDirectory::ensure(&temporary.path().join(STATE_DIRECTORY)).unwrap();
    let path = root.path().join(LIFECYCLE_LOCK_FILE);
    let blocker =
        PrivateFileLock::try_acquire_in(root.try_clone().unwrap(), &path, SWEEP_LOCK_MAX_BYTES)
            .unwrap()
            .unwrap();

    assert!(LifecycleLock::try_acquire(&root).unwrap().is_none());
    drop(blocker);
    drop(LifecycleLock::try_acquire(&root).unwrap().unwrap());
}
