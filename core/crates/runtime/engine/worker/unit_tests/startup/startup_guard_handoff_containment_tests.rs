use super::StartupChildGuard;
use crate::process::ProcessIdentity;
use crate::worker::pty_fault_reaper::{FaultReapReceiver, FaultReaperReservation};
use std::process::Command;

const TEST_CHILD_EXECUTABLE: &str = "/bin/sleep";
const TEST_CHILD_DURATION: &str = "30";
const HANDOFF_TIMEOUT: std::time::Duration =
    crate::worker::termination_timing::WORKER_TERMINATION_TIMEOUT;

#[test]
fn tracked_handoff_observer_panic_preserves_deferred_custody() {
    let (mut guard, root) = spawn_guard();
    guard.track(root).unwrap();
    let completion = install_disconnected_observer(&mut guard);

    drop(guard);

    assert_waited(completion);
    assert!(!root.matches());
}

#[test]
fn fresh_handoff_observer_panic_preserves_deferred_custody() {
    let (mut guard, root) = spawn_guard();
    let completion = install_disconnected_observer(&mut guard);

    drop(guard);

    assert_waited(completion);
    assert!(!root.matches());
}

#[test]
fn tracked_accept_panic_retries_pending_handoff() {
    let (mut guard, root) = spawn_guard();
    guard.track(root).unwrap();
    let completion = install_accept_panic(&mut guard);

    drop(guard);

    assert_waited(completion);
    assert!(!root.matches());
}

#[test]
fn fresh_accept_panic_retries_pending_handoff() {
    let (mut guard, root) = spawn_guard();
    let completion = install_accept_panic(&mut guard);

    drop(guard);

    assert_waited(completion);
    assert!(!root.matches());
}

#[test]
fn fresh_claim_driver_panic_retries_retained_custody() {
    let (mut guard, root) = spawn_guard();
    let completion = install_drive_panic(&mut guard);

    drop(guard);

    let outcome = assert_waited(completion);
    assert!(outcome.fault.is_some());
    assert!(!root.matches());
}

fn spawn_guard() -> (StartupChildGuard, ProcessIdentity) {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(TEST_CHILD_EXECUTABLE).arg(TEST_CHILD_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    (StartupChildGuard::new(Box::new(child)), root)
}

fn install_disconnected_observer(guard: &mut StartupChildGuard) -> FaultReapReceiver {
    let (mut reservation, observations) =
        FaultReaperReservation::reserve_observed_for_test().unwrap();
    let completion = reservation.observe_startup_completion();
    drop(observations);
    drop(guard.fault_reaper.replace(reservation));
    completion
}

fn install_accept_panic(guard: &mut StartupChildGuard) -> FaultReapReceiver {
    let mut reservation = FaultReaperReservation::reserve().unwrap();
    let completion = reservation.observe_startup_completion();
    reservation.panic_on_startup_accept_for_test();
    drop(guard.fault_reaper.replace(reservation));
    completion
}

fn install_drive_panic(guard: &mut StartupChildGuard) -> FaultReapReceiver {
    let mut reservation = FaultReaperReservation::reserve().unwrap();
    let completion = reservation.observe_startup_completion();
    reservation.panic_on_startup_drive_for_test();
    drop(guard.fault_reaper.replace(reservation));
    completion
}

fn assert_waited(completion: FaultReapReceiver) -> crate::worker::pty_ownership::ReapOutcome {
    let outcome = completion.receive_timeout(HANDOFF_TIMEOUT).unwrap();
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    assert!(outcome.status.is_some());
    outcome
}
