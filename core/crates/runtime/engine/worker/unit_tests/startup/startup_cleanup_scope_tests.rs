use super::*;
use crate::worker::pty_output_progress::ExclusivePtyOutput;
use crate::worker::startup::startup_guard::StartupOutputProgress;
use crate::worker::startup::startup_untracked_reaper::{
    StartupUntrackedCleanup, StartupUntrackedReaperReservation,
};
#[path = "../../test_support/unit/startup/startup_untracked_reaper_support.rs"]
mod support;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use support::{ChildObservation, GatedChild};

const ASYNC_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(2);
const SETTLEMENT_PROBE: Duration = Duration::from_millis(25);
const DROP_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(2);

#[tokio::test]
async fn no_handoff_settles_immediately() {
    let mut scope = StartupCleanupScope::new().unwrap();
    let reservation = StartupUntrackedReaperReservation::reserve(&mut scope).unwrap();
    drop(reservation);

    tokio::time::timeout(ASYNC_SETTLEMENT_TIMEOUT, scope.settle())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn handoff_blocks_scope_until_child_settlement() {
    let observation = ChildObservation::new();
    let mut scope = StartupCleanupScope::new().unwrap();
    handoff_gated(&mut scope, &observation);
    let mut settlement = tokio::spawn(scope.settle());

    assert!(
        tokio::time::timeout(SETTLEMENT_PROBE, &mut settlement)
            .await
            .is_err()
    );
    observation.settled.store(true, Ordering::Release);
    settlement.await.unwrap().unwrap();
}

#[tokio::test]
async fn reaper_panic_is_reported_by_scope() {
    let mut scope = StartupCleanupScope::new().unwrap();
    let reservation = StartupUntrackedReaperReservation::reserve_panicking(&mut scope).unwrap();
    drop(reservation);

    assert!(scope.settle().await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_settlement_waits_for_handed_off_custody() {
    let observation = ChildObservation::new();
    let mut scope = StartupCleanupScope::new().unwrap();
    handoff_gated(&mut scope, &observation);
    let mut settlement = tokio::spawn(scope.settle());

    assert!(
        tokio::time::timeout(SETTLEMENT_PROBE, &mut settlement)
            .await
            .is_err()
    );
    settlement.abort();
    assert!(settlement.await.unwrap_err().is_cancelled());
    observation.settled.store(true, Ordering::Release);
    wait_until(|| observation.dropped.load(Ordering::Acquire));
}

#[test]
fn scope_drop_returns_without_releasing_handed_off_custody() {
    let observation = ChildObservation::new();
    let mut scope = StartupCleanupScope::new().unwrap();
    handoff_gated(&mut scope, &observation);
    let (completed, completion) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        drop(scope);
        let _ = completed.send(());
    });

    completion.recv_timeout(SETTLEMENT_PROBE).unwrap();
    assert!(!observation.dropped.load(Ordering::Acquire));
    observation.settled.store(true, Ordering::Release);
    wait_until(|| observation.dropped.load(Ordering::Acquire));
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + DROP_SETTLEMENT_TIMEOUT;
    while !predicate() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(SETTLEMENT_PROBE);
    }
}

fn handoff_gated(scope: &mut StartupCleanupScope, observation: &ChildObservation) {
    let reservation = StartupUntrackedReaperReservation::reserve(scope).unwrap();
    let output_calls = Arc::new(AtomicUsize::new(usize::default()));
    reservation.handoff(StartupUntrackedCleanup::closed_gate_for_test(
        Box::new(GatedChild::observed(observation)),
        StartupOutputProgress::Exclusive(ExclusivePtyOutput::observed(output_calls)),
    ));
}
