use super::*;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const FIRST_WAITER: usize = 1;
const SECOND_WAITER: usize = 2;
const STALE_PROCESS_ID_OFFSET: u32 = 1;
const PROGRESS_DELAY: Duration = Duration::from_millis(300);
const PROGRESS_IDLE_TIMEOUT: Duration = Duration::from_millis(500);
const CONTROLLED_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const STALL_TIMEOUT: Duration = Duration::from_millis(50);
const TEST_TIMEOUT: Duration = Duration::from_secs(2);
const HEALTHY_QUEUE_ERROR: &str = "healthy queue must keep making progress";
const WAITER_QUEUE_ERROR: &str = "waiter did not enter FIFO queue";

#[test]
fn fifo_turns_advance_while_total_wait_exceeds_the_idle_timeout() {
    let key = identity();
    let owner = ProcessLockTurn::try_acquire(key).unwrap().unwrap();
    let (acquired_tx, acquired_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let first = spawn_waiter(key, FIRST_WAITER, acquired_tx.clone(), release_rx);
    wait_for_waiters(key, FIRST_WAITER);
    let (second_release_tx, second_release_rx) = mpsc::channel();
    let second = spawn_waiter(key, SECOND_WAITER, acquired_tx, second_release_rx);
    wait_for_waiters(key, SECOND_WAITER);

    std::thread::sleep(PROGRESS_DELAY);
    drop(owner);
    assert_eq!(
        acquired_rx.recv_timeout(TEST_TIMEOUT).unwrap(),
        FIRST_WAITER
    );
    std::thread::sleep(PROGRESS_DELAY);
    release_tx.send(()).unwrap();
    assert_eq!(
        acquired_rx.recv_timeout(TEST_TIMEOUT).unwrap(),
        SECOND_WAITER
    );
    second_release_tx.send(()).unwrap();
    first.join().unwrap();
    second.join().unwrap();
}

#[test]
fn try_acquire_cannot_barge_between_owner_release_and_waiter_notification() {
    let key = identity();
    let owner = ProcessLockTurn::try_acquire(key).unwrap().unwrap();
    let (acquired_tx, acquired_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let waiter = spawn_waiter(key, FIRST_WAITER, acquired_tx, release_rx);
    wait_for_waiters(key, FIRST_WAITER);

    let signal = owner.release_before_notification_for_test();
    assert!(ProcessLockTurn::try_acquire(key).unwrap().is_none());
    super::turn::notify_waiters_for_test(&signal);
    assert_eq!(
        acquired_rx.recv_timeout(TEST_TIMEOUT).unwrap(),
        FIRST_WAITER
    );
    release_tx.send(()).unwrap();
    waiter.join().unwrap();
}

#[test]
fn timed_out_front_waiter_is_removed_and_the_successor_advances() {
    let key = identity();
    let owner = ProcessLockTurn::try_acquire(key).unwrap().unwrap();
    let (cancelled_tx, cancelled_rx) = mpsc::channel();
    let cancelled = std::thread::spawn(move || {
        let result = ProcessLockTurn::acquire(key, CONTROLLED_IDLE_TIMEOUT).unwrap();
        cancelled_tx.send(result.is_none()).unwrap();
    });
    wait_for_waiters(key, FIRST_WAITER);
    let (acquired_tx, acquired_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let successor = spawn_waiter(key, SECOND_WAITER, acquired_tx, release_rx);
    wait_for_waiters(key, SECOND_WAITER);

    super::turn::expire_front_waiter_for_test(key);
    assert!(cancelled_rx.recv_timeout(TEST_TIMEOUT).unwrap());
    cancelled.join().unwrap();
    wait_for_waiters(key, FIRST_WAITER);
    drop(owner);
    assert_eq!(
        acquired_rx.recv_timeout(TEST_TIMEOUT).unwrap(),
        SECOND_WAITER
    );
    release_tx.send(()).unwrap();
    successor.join().unwrap();
    assert!(ProcessLockTurn::try_acquire(key).unwrap().is_some());
}

#[test]
fn absolute_deadline_cancels_the_waiter_without_releasing_the_owner() {
    let key = identity();
    let owner = ProcessLockTurn::try_acquire(key).unwrap().unwrap();
    let deadline = Instant::now() + STALL_TIMEOUT;

    assert!(
        ProcessLockTurn::acquire_until(key, deadline)
            .unwrap()
            .is_none()
    );
    assert!(ProcessLockTurn::try_acquire(key).unwrap().is_none());
    drop(owner);
    assert!(ProcessLockTurn::try_acquire(key).unwrap().is_some());
}

#[test]
fn stale_or_unregistered_turn_tokens_cannot_release_an_owner() {
    let stale_key = identity();
    let stale_process_id = std::process::id()
        .checked_add(STALE_PROCESS_ID_OFFSET)
        .unwrap();
    drop(ProcessLockTurn::new(stale_key, stale_process_id));
    assert!(!current_registry().turns.contains_key(&stale_key));

    let unregistered_key = identity();
    drop(ProcessLockTurn::new(unregistered_key, std::process::id()));
    assert!(!current_registry().turns.contains_key(&unregistered_key));
}

#[test]
fn cancelling_the_only_unowned_waiter_removes_its_turn_state() {
    let key = identity();
    let queued = super::turn::insert_unowned_waiter_for_test(key);
    let mut registry = current_registry();

    super::turn::cancel_waiter(&mut registry, key, &queued);

    assert!(!registry.turns.contains_key(&key));
}

fn spawn_waiter(
    key: ObjectIdentity,
    order: usize,
    acquired: mpsc::Sender<usize>,
    release: mpsc::Receiver<()>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let turn = ProcessLockTurn::acquire(key, PROGRESS_IDLE_TIMEOUT)
            .unwrap()
            .expect(HEALTHY_QUEUE_ERROR);
        acquired.send(order).unwrap();
        release.recv_timeout(TEST_TIMEOUT).unwrap();
        drop(turn);
    })
}

fn wait_for_waiters(key: ObjectIdentity, expected: usize) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let count = ProcessLockTurn::queued_waiters_for_test(key);
        if count == expected {
            return;
        }
        assert!(Instant::now() < deadline, "{WAITER_QUEUE_ERROR}");
        std::thread::yield_now();
    }
}

fn identity() -> ObjectIdentity {
    let directory = tempfile::tempdir().unwrap();
    ObjectIdentity::from_metadata(&directory.path().metadata().unwrap())
}
