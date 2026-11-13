use super::*;
use crate::paths::directory::PrivateDirectory;
use crate::paths::private_file_lock::PrivateFileLock;
use std::sync::{Arc, Barrier};

const CONCURRENT_ATTEMPTS: usize = 8;
const LOCK_RECORD_LIMIT: u64 = MAX_COORDINATION_LOCK_BYTES;
const MAX_BACKGROUND_CHUNK_ENTRIES: usize = 16;
const MAX_BACKGROUND_CHUNK_DELETIONS: usize = 4;

struct Fixture {
    _root: tempfile::TempDir,
    directory: PrivateDirectory,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("coordination");
        let directory = PrivateDirectory::ensure(&path).unwrap();
        Self {
            _root: root,
            directory,
        }
    }

    fn candidate(&self, index: usize) -> PathBuf {
        candidate_path(self.directory.path(), index)
    }

    fn stale(&self, index: usize) -> PathBuf {
        let path = self.candidate(index);
        let lock = PrivateFileLock::try_acquire(&path, LOCK_RECORD_LIMIT)
            .unwrap()
            .unwrap();
        drop(lock);
        path
    }

    fn stale_socket_maintenance_owner(&self) -> PathBuf {
        let digest = lock_digest(
            CoordinationScope::SocketMaintenance,
            self.directory.path(),
            None,
        );
        let path = self.directory.path().join(format!(
            "{SOCKET_MAINTENANCE_SCOPE}-{digest}{LOCK_FILE_EXTENSION}"
        ));
        drop(
            PrivateFileLock::try_acquire(&path, LOCK_RECORD_LIMIT)
                .unwrap()
                .unwrap(),
        );
        path
    }
}

pub(super) fn candidate_path(root: &Path, index: usize) -> PathBuf {
    let discriminator = index.to_string();
    let digest = lock_digest(CoordinationScope::Worker, root, Some(&discriminator));
    root.join(format!("{WORKER_SCOPE}-{digest}{LOCK_FILE_EXTENSION}"))
}

#[test]
fn background_chunk_budget_remains_responsive_to_cancellation() {
    const {
        assert!(SWEEP_ENTRY_BUDGET <= MAX_BACKGROUND_CHUNK_ENTRIES);
        assert!(SWEEP_DELETE_BUDGET <= MAX_BACKGROUND_CHUNK_DELETIONS);
    }
}

#[test]
fn drop_removes_the_owned_lock_without_shared_retirement_state() {
    let fixture = Fixture::new();
    let path = fixture.candidate(0);
    let lock = CoordinationLock::try_acquire(&path, LOCK_RECORD_LIMIT)
        .unwrap()
        .unwrap();
    assert!(path.is_file());
    drop(lock);
    assert!(!path.exists());
    let unrelated = fixture.directory.path().join("unrelated-poison");
    std::fs::create_dir(&unrelated).unwrap();
    let second = fixture.candidate(9);
    drop(
        CoordinationLock::try_acquire(&second, LOCK_RECORD_LIMIT)
            .unwrap()
            .unwrap(),
    );
    assert!(!second.exists());
    assert!(unrelated.is_dir());
}

#[test]
fn one_key_has_exactly_one_concurrent_owner() {
    let fixture = Fixture::new();
    let path = fixture.candidate(1);
    let start = Arc::new(Barrier::new(CONCURRENT_ATTEMPTS + 1));
    let finish = Arc::new(Barrier::new(CONCURRENT_ATTEMPTS + 1));
    let handles: Vec<_> = (0..CONCURRENT_ATTEMPTS)
        .map(|_| {
            let path = path.clone();
            let start = Arc::clone(&start);
            let finish = Arc::clone(&finish);
            std::thread::spawn(move || {
                start.wait();
                let guard = CoordinationLock::try_acquire(&path, LOCK_RECORD_LIMIT).unwrap();
                let acquired = guard.is_some();
                finish.wait();
                drop(guard);
                acquired
            })
        })
        .collect();
    start.wait();
    finish.wait();
    let owners = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(|acquired| *acquired)
        .count();
    assert_eq!(owners, 1);
    assert!(!path.exists());
}

#[test]
fn live_file_lock_is_preserved_until_its_owner_exits() {
    let fixture = Fixture::new();
    let path = fixture.stale(2);
    let live = PrivateFileLock::try_acquire_existing(&path, LOCK_RECORD_LIMIT)
        .unwrap()
        .unwrap();
    scavenge::sweep(fixture.directory.path()).unwrap();
    assert!(path.exists());
    drop(live);
    scavenge::sweep(fixture.directory.path()).unwrap();
    assert!(!path.exists());
}

#[test]
fn socket_isolation_stale_per_root_owner_is_scavenged() {
    let fixture = Fixture::new();
    let path = fixture.stale_socket_maintenance_owner();

    scavenge::sweep(fixture.directory.path()).unwrap();

    assert!(!path.exists());
}
