use super::*;
use crate::paths::constants::PRIVATE_FILE_MODE;
use crate::paths::private_file_lock::PrivateFileLock;
use std::fs::{self, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::mpsc;

const INJECTED_LOCK_FAILURE: &str = "injected post-flock failure";
const INJECTED_REGISTRY_PANIC: &str = "injected registry panic";
const MISSING_INJECTED_FAILURE: &str = "post-flock hook unexpectedly succeeded";
const FOREIGN_PROCESS_INCREMENT: u32 = 1;
const EMPTY_LOCK_MAX_BYTES: u64 = 0;
const LOCK_FILE_NAME: &str = "process.lock";

#[test]
fn shared_locks_coexist_and_exclusive_waits_for_the_last_drop() {
    let fixture = Fixture::new();
    let first = acquire(fixture.identity(), Mode::Shared).unwrap();
    let second = acquire(fixture.identity(), Mode::Shared).unwrap();
    assert!(acquire(fixture.identity(), Mode::Exclusive).is_none());
    drop(first);
    assert!(acquire(fixture.identity(), Mode::Exclusive).is_none());
    drop(second);
    assert!(acquire(fixture.identity(), Mode::Exclusive).is_some());
}

#[test]
fn exclusive_conflicts_with_every_mode_and_failed_acquisitions_leave_no_owner() {
    let fixture = Fixture::new();
    let exclusive = acquire(fixture.identity(), Mode::Exclusive).unwrap();
    assert!(acquire(fixture.identity(), Mode::Exclusive).is_none());
    assert!(acquire(fixture.identity(), Mode::Shared).is_none());
    drop(exclusive);
    let shared = acquire(fixture.identity(), Mode::Shared).unwrap();
    drop(shared);
    assert!(acquire(fixture.identity(), Mode::Exclusive).is_some());
}

#[test]
fn cross_thread_contention_uses_the_same_registry() {
    let fixture = Fixture::new();
    let identity = fixture.identity();
    let exclusive = acquire(identity, Mode::Exclusive).unwrap();
    let (sender, receiver) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        sender
            .send(acquire(identity, Mode::Shared).is_none())
            .unwrap();
    });
    assert!(receiver.recv().unwrap());
    thread.join().unwrap();
    drop(exclusive);
}

#[test]
fn inode_replacement_has_an_independent_registry_identity() {
    let fixture = Fixture::new();
    let original = acquire(fixture.identity(), Mode::Exclusive).unwrap();
    fs::remove_file(&fixture.path).unwrap();
    fixture.create();
    let replacement = fixture.identity();
    assert!(acquire(replacement, Mode::Exclusive).is_some());
    drop(original);
}

#[test]
fn post_flock_failure_releases_the_process_registry_entry() {
    let fixture = Fixture::new();
    let failed =
        PrivateFileLock::try_acquire_with_hook(&fixture.path, EMPTY_LOCK_MAX_BYTES, || {
            anyhow::bail!(INJECTED_LOCK_FAILURE)
        });
    assert!(
        failed
            .err()
            .expect(MISSING_INJECTED_FAILURE)
            .to_string()
            .contains(INJECTED_LOCK_FAILURE)
    );
    assert!(
        PrivateFileLock::try_acquire(&fixture.path, EMPTY_LOCK_MAX_BYTES)
            .unwrap()
            .is_some()
    );
}

#[test]
fn poisoned_registry_is_recovered_without_losing_future_locks() {
    let thread = std::thread::spawn(|| {
        let _registry = current_registry();
        panic!("{INJECTED_REGISTRY_PANIC}");
    });
    assert!(thread.join().is_err());
    let fixture = Fixture::new();
    assert!(acquire(fixture.identity(), Mode::Exclusive).is_some());
}

#[test]
fn drop_ignores_foreign_process_copies_and_absent_registry_entries() {
    let fixture = Fixture::new();
    let identity = fixture.identity().object_identity();
    let owner = acquire(fixture.identity(), Mode::Exclusive).unwrap();
    let foreign_process_id = std::process::id()
        .checked_add(FOREIGN_PROCESS_INCREMENT)
        .unwrap();
    drop(ProcessFileLock::detached_logical_for_test(
        identity,
        Mode::Exclusive,
        foreign_process_id,
    ));
    assert!(acquire(fixture.identity(), Mode::Exclusive).is_none());
    drop(owner);

    drop(ProcessFileLock::detached_logical_for_test(
        identity,
        Mode::Exclusive,
        std::process::id(),
    ));
    assert!(acquire(fixture.identity(), Mode::Exclusive).is_some());
}

fn acquire(identity: FileIdentity, mode: Mode) -> Option<ProcessFileLock> {
    ProcessFileLock::try_acquire(identity, mode).unwrap()
}

struct Fixture {
    _temporary: tempfile::TempDir,
    path: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join(LOCK_FILE_NAME);
        let fixture = Self {
            _temporary: temporary,
            path,
        };
        fixture.create();
        fixture
    }

    fn create(&self) {
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(PRIVATE_FILE_MODE)
            .open(&self.path)
            .unwrap();
    }

    fn identity(&self) -> FileIdentity {
        FileIdentity::inspect_bounded(
            &fs::metadata(&self.path).unwrap(),
            &self.path,
            EMPTY_LOCK_MAX_BYTES,
        )
        .unwrap()
    }
}
