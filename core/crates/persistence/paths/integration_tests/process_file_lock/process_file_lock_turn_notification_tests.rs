use super::*;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const FIRST_WAITER_COUNT: usize = 1;
const SECOND_WAITER_COUNT: usize = 2;
const FRONT_WAIT_TIMEOUT: Duration = Duration::from_millis(500);
const SUCCESSOR_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const CONTROLLED_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const TEST_DEADLINE_OVERFLOW: &str = "turn notification test deadline overflow";
const WAITER_QUEUE_TIMEOUT: &str = "waiters did not enter the turn queue";

#[test]
fn front_timeout_notifies_unowned_successor() {
    let directory = tempfile::tempdir().unwrap();
    let key = ObjectIdentity::from_metadata(&directory.path().metadata().unwrap());
    let owner = ProcessLockTurn::try_acquire(key).unwrap().unwrap();
    let (front_done_tx, front_done_rx) = mpsc::channel();
    let front = std::thread::spawn(move || {
        let deadline = Instant::now()
            .checked_add(FRONT_WAIT_TIMEOUT)
            .expect(TEST_DEADLINE_OVERFLOW);
        front_done_tx
            .send(ProcessLockTurn::acquire_until(key, deadline).unwrap())
            .unwrap();
    });
    wait_for_waiters(key, FIRST_WAITER_COUNT);
    let (successor_tx, successor_rx) = mpsc::channel();
    let successor = std::thread::spawn(move || {
        let deadline = Instant::now()
            .checked_add(SUCCESSOR_WAIT_TIMEOUT)
            .expect(TEST_DEADLINE_OVERFLOW);
        successor_tx
            .send(ProcessLockTurn::acquire_until(key, deadline).unwrap())
            .unwrap();
    });
    wait_for_waiters(key, SECOND_WAITER_COUNT);

    let _delayed_notification = owner.release_before_notification_for_test();
    assert!(front_done_rx.recv_timeout(TEST_TIMEOUT).unwrap().is_none());
    front.join().unwrap();
    let successor_turn = successor_rx.recv_timeout(TEST_TIMEOUT).unwrap().unwrap();
    drop(successor_turn);
    successor.join().unwrap();
}

#[test]
fn late_owner_release_cannot_revive_an_expired_idle_waiter() {
    let directory = tempfile::tempdir().unwrap();
    let key = ObjectIdentity::from_metadata(&directory.path().metadata().unwrap());
    let owner = ProcessLockTurn::try_acquire(key).unwrap().unwrap();
    let (front_tx, front_rx) = mpsc::channel();
    let front = std::thread::spawn(move || {
        front_tx
            .send(ProcessLockTurn::acquire(key, FRONT_WAIT_TIMEOUT).unwrap())
            .unwrap();
    });
    wait_for_waiters(key, FIRST_WAITER_COUNT);
    let (successor_tx, successor_rx) = mpsc::channel();
    let successor = std::thread::spawn(move || {
        successor_tx
            .send(ProcessLockTurn::acquire(key, SUCCESSOR_WAIT_TIMEOUT).unwrap())
            .unwrap();
    });
    wait_for_waiters(key, SECOND_WAITER_COUNT);

    let late_progress = Instant::now()
        .checked_add(FRONT_WAIT_TIMEOUT)
        .expect(TEST_DEADLINE_OVERFLOW);
    let signal = owner.release_at_before_notification_for_test(late_progress);
    super::turn::notify_waiters_for_test(&signal);
    let front_result = front_rx.recv_timeout(TEST_TIMEOUT).unwrap();
    let front_timed_out = front_result.is_none();
    drop(front_result);
    front.join().unwrap();
    let successor_turn = successor_rx.recv_timeout(TEST_TIMEOUT).unwrap().unwrap();
    drop(successor_turn);
    successor.join().unwrap();

    assert!(front_timed_out);
}

#[test]
fn idle_deadline_overflow_fails_without_leaking_the_turn_queue() {
    let directory = tempfile::tempdir().unwrap();
    let key = ObjectIdentity::from_metadata(&directory.path().metadata().unwrap());

    assert!(ProcessLockTurn::acquire(key, Duration::MAX).is_err());
    assert!(ProcessLockTurn::try_acquire(key).unwrap().is_some());
}

#[test]
fn renewal_overflow_cancels_front_and_notifies_unowned_successor() {
    let directory = tempfile::tempdir().unwrap();
    let key = ObjectIdentity::from_metadata(&directory.path().metadata().unwrap());
    let owner = ProcessLockTurn::try_acquire(key).unwrap().unwrap();
    let (front_tx, front_rx) = mpsc::channel();
    let front = std::thread::spawn(move || {
        front_tx
            .send(ProcessLockTurn::acquire(key, CONTROLLED_IDLE_TIMEOUT).is_err())
            .unwrap();
    });
    wait_for_waiters(key, FIRST_WAITER_COUNT);
    let (successor_tx, successor_rx) = mpsc::channel();
    let successor = std::thread::spawn(move || {
        let deadline = Instant::now()
            .checked_add(SUCCESSOR_WAIT_TIMEOUT)
            .expect(TEST_DEADLINE_OVERFLOW);
        successor_tx
            .send(ProcessLockTurn::acquire_until(key, deadline).unwrap())
            .unwrap();
    });
    wait_for_waiters(key, SECOND_WAITER_COUNT);

    super::turn::arm_front_renewal_overflow_for_test(key);
    drop(owner);
    assert!(front_rx.recv_timeout(TEST_TIMEOUT).unwrap());
    front.join().unwrap();
    let successor_turn = successor_rx.recv_timeout(TEST_TIMEOUT).unwrap().unwrap();
    drop(successor_turn);
    successor.join().unwrap();

    assert!(ProcessLockTurn::try_acquire(key).unwrap().is_some());
}

fn wait_for_waiters(key: ObjectIdentity, expected: usize) {
    let deadline = Instant::now()
        .checked_add(TEST_TIMEOUT)
        .expect(TEST_DEADLINE_OVERFLOW);
    while ProcessLockTurn::queued_waiters_for_test(key) != expected {
        assert!(Instant::now() < deadline, "{WAITER_QUEUE_TIMEOUT}");
        std::thread::yield_now();
    }
}
