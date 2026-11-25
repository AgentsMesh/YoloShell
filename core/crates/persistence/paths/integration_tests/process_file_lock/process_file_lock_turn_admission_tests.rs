use super::*;
use std::sync::mpsc;
use std::time::Duration;

const IDLE_TIMEOUT: Duration = Duration::from_millis(10);
const REGISTRY_HOLD_TIME: Duration = Duration::from_millis(100);
const TEST_TIMEOUT: Duration = Duration::from_secs(2);
const ADMISSION_REACHED_TIMEOUT: &str = "turn acquire did not reach registry admission";
const ACQUIRE_RESULT_TIMEOUT: &str = "turn acquire result timed out";
const IMMEDIATE_GRANT_MISSING: &str = "an idle turn must be granted immediately";
const CONTENDED_GRANT_MISSING: &str = "an idle turn must outlive unrelated registry contention";

#[test]
fn zero_idle_timeout_performs_one_immediate_turn_attempt() {
    let key = identity();

    let turn = ProcessLockTurn::acquire(key, Duration::ZERO)
        .unwrap()
        .expect(IMMEDIATE_GRANT_MISSING);

    drop(turn);
    assert!(ProcessLockTurn::try_acquire(key).unwrap().is_some());
}

#[test]
fn zero_idle_timeout_cancels_cleanly_when_the_turn_is_owned() {
    let key = identity();
    let owner = ProcessLockTurn::try_acquire(key).unwrap().unwrap();

    assert!(
        ProcessLockTurn::acquire(key, Duration::ZERO)
            .unwrap()
            .is_none()
    );

    drop(owner);
    assert!(ProcessLockTurn::try_acquire(key).unwrap().is_some());
}

#[test]
fn registry_contention_does_not_consume_the_idle_timeout() {
    let key = identity();
    let registry = current_registry();
    let (admitted_tx, admitted_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    let waiter = std::thread::spawn(move || {
        ProcessLockTurn::set_registry_admission_hook(move || admitted_tx.send(()).unwrap());
        result_tx
            .send(ProcessLockTurn::acquire(key, IDLE_TIMEOUT).unwrap())
            .unwrap();
    });
    admitted_rx
        .recv_timeout(TEST_TIMEOUT)
        .expect(ADMISSION_REACHED_TIMEOUT);

    std::thread::sleep(REGISTRY_HOLD_TIME);
    drop(registry);

    let turn = result_rx
        .recv_timeout(TEST_TIMEOUT)
        .expect(ACQUIRE_RESULT_TIMEOUT)
        .expect(CONTENDED_GRANT_MISSING);
    drop(turn);
    waiter.join().unwrap();
}

fn identity() -> ObjectIdentity {
    let directory = tempfile::tempdir().unwrap();
    ObjectIdentity::from_metadata(&directory.path().metadata().unwrap())
}
