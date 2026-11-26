use super::*;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[path = "../../test_support/unit/private_file_lock_wait_test_support.rs"]
mod support;
use support::{Fixture, create_lock_file, spawn_waiter, wait_for_queued, wait_lock};

include!("private_file_lock_version_tests.rs");
include!("private_file_lock_deadline_tests.rs");

const BOUNDED_TIMEOUT: Duration = Duration::from_millis(40);
const CONTENTION_TIMEOUT: Duration = Duration::from_millis(500);
const RETRY_DELAY: Duration = Duration::from_millis(2);
const TEST_TIMEOUT: Duration = Duration::from_secs(2);
const FIRST_WAITER: usize = 1;
const SECOND_WAITER: usize = 2;
const FLOCK_SUCCESS: libc::c_int = 0;
const INITIAL_LOCK_CONTENT: &[u8] = b"a";
const LOCK_MAX_BYTES: u64 = 0;
const LOCK_FILE_NAME: &str = "bounded-private.lock";
const DISPLACED_FILE_NAME: &str = "displaced-private.lock";
const MUTABLE_LOCK_MAX_BYTES: u64 = 1;
const REPLACEMENT_LOCK_CONTENT: &[u8] = b"b";

#[test]
fn zero_retry_delay_is_rejected_without_acquiring_the_lock() {
    let fixture = Fixture::new();
    let parent = PrivateDirectory::open(fixture.path.parent().unwrap()).unwrap();

    assert!(
        PrivateFileLock::acquire_in_wait(
            parent,
            &fixture.path,
            LOCK_MAX_BYTES,
            CONTENTION_TIMEOUT,
            Duration::ZERO,
        )
        .is_err()
    );
    assert!(
        PrivateFileLock::try_acquire(&fixture.path, LOCK_MAX_BYTES)
            .unwrap()
            .is_some()
    );
}

#[test]
fn kernel_flock_contention_is_retried_until_the_holder_releases() {
    let fixture = Fixture::new();
    let holder = fixture.kernel_holder();
    let (observed_tx, observed_rx) = mpsc::channel();
    let path = fixture.path.clone();
    let waiter = std::thread::spawn(move || {
        PrivateFileLock::set_wait_contention_hook(move || observed_tx.send(()).unwrap());
        wait_lock(&path, CONTENTION_TIMEOUT).map(drop)
    });

    observed_rx.recv_timeout(TEST_TIMEOUT).unwrap();
    drop(holder);

    waiter.join().unwrap().unwrap();
}

#[test]
fn bounded_wait_times_out_without_stealing_or_leaking_ownership() {
    let fixture = Fixture::new();
    let holder = fixture.kernel_holder();
    let started = Instant::now();

    let result = wait_lock(&fixture.path, BOUNDED_TIMEOUT);
    let elapsed = started.elapsed();

    assert!(result.is_err());
    assert!(elapsed >= BOUNDED_TIMEOUT);
    assert!(elapsed < TEST_TIMEOUT);
    assert!(
        PrivateFileLock::try_acquire(&fixture.path, LOCK_MAX_BYTES)
            .unwrap()
            .is_none()
    );
    drop(holder);
    assert!(
        PrivateFileLock::try_acquire(&fixture.path, LOCK_MAX_BYTES)
            .unwrap()
            .is_some()
    );
}

#[test]
fn waited_lock_revalidates_the_named_path_identity() {
    let fixture = Fixture::new();
    let holder = fixture.kernel_holder();
    let (observed_tx, observed_rx) = mpsc::channel();
    let path = fixture.path.clone();
    let waiter_path = path.clone();
    let waiter = std::thread::spawn(move || {
        PrivateFileLock::set_wait_contention_hook(move || observed_tx.send(()).unwrap());
        wait_lock(&waiter_path, CONTENTION_TIMEOUT)
    });
    observed_rx.recv_timeout(TEST_TIMEOUT).unwrap();
    std::fs::rename(&path, path.with_file_name(DISPLACED_FILE_NAME)).unwrap();
    create_lock_file(&path);
    drop(holder);

    assert!(waiter.join().unwrap().is_err());
    assert!(
        PrivateFileLock::try_acquire(&path, LOCK_MAX_BYTES)
            .unwrap()
            .is_some()
    );
}

#[test]
fn waited_lock_rejects_a_changed_content_version() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(LOCK_FILE_NAME);
    let mut initial = PrivateFileLock::try_acquire(&path, MUTABLE_LOCK_MAX_BYTES)
        .unwrap()
        .unwrap();
    initial.replace_contents(INITIAL_LOCK_CONTENT).unwrap();
    drop(initial);
    let entry = PrivateFileEntry::open(&path).unwrap();
    let opened = entry.open_rw(MUTABLE_LOCK_MAX_BYTES).unwrap();
    assert_eq!(
        unsafe { libc::flock(opened.file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        FLOCK_SUCCESS
    );
    let holder = opened.file;
    let (observed_tx, observed_rx) = mpsc::channel();
    let waiter_path = path.clone();
    let waiter = std::thread::spawn(move || {
        PrivateFileLock::set_wait_contention_hook(move || observed_tx.send(()).unwrap());
        let directory = PrivateDirectory::open(waiter_path.parent().unwrap()).unwrap();
        PrivateFileLock::acquire_in_wait(
            directory,
            &waiter_path,
            MUTABLE_LOCK_MAX_BYTES,
            CONTENTION_TIMEOUT,
            RETRY_DELAY,
        )
    });
    observed_rx.recv_timeout(TEST_TIMEOUT).unwrap();
    std::fs::write(&path, REPLACEMENT_LOCK_CONTENT).unwrap();
    drop(holder);
    assert!(waiter.join().unwrap().is_err());
    assert!(
        PrivateFileLock::try_acquire(&path, MUTABLE_LOCK_MAX_BYTES)
            .unwrap()
            .is_some()
    );
}

#[test]
fn same_process_waiters_acquire_the_lock_in_fifo_order() {
    let fixture = Fixture::new();
    let holder = PrivateFileLock::try_acquire(&fixture.path, LOCK_MAX_BYTES)
        .unwrap()
        .unwrap();
    let identity = crate::paths::identity::ObjectIdentity::from_file(
        &std::fs::File::open(&fixture.path).unwrap(),
    )
    .unwrap()
    .unwrap();
    let (acquired_tx, acquired_rx) = mpsc::channel();
    let (first_release_tx, first_release_rx) = mpsc::channel();
    let first = spawn_waiter(
        fixture.path.clone(),
        FIRST_WAITER,
        acquired_tx.clone(),
        first_release_rx,
    );
    wait_for_queued(identity, FIRST_WAITER);
    let (second_release_tx, second_release_rx) = mpsc::channel();
    let second = spawn_waiter(
        fixture.path.clone(),
        SECOND_WAITER,
        acquired_tx,
        second_release_rx,
    );
    wait_for_queued(identity, SECOND_WAITER);

    drop(holder);
    assert_eq!(
        acquired_rx.recv_timeout(TEST_TIMEOUT).unwrap(),
        FIRST_WAITER
    );
    first_release_tx.send(()).unwrap();
    assert_eq!(
        acquired_rx.recv_timeout(TEST_TIMEOUT).unwrap(),
        SECOND_WAITER
    );
    second_release_tx.send(()).unwrap();
    first.join().unwrap();
    second.join().unwrap();
}
