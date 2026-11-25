use super::*;
use crate::worker::pty_output_progress::{ExclusivePtyOutput, ReaderBackedPtyOutput};
#[path = "../../test_support/unit/startup/startup_untracked_reaper_scripted_support.rs"]
mod scripted_support;
#[path = "../../test_support/unit/startup/startup_untracked_reaper_support.rs"]
mod support;
use scripted_support::ScriptedChild;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use support::{ChildObservation, GatedChild};

const TEST_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(2);
const TEST_WAIT_INTERVAL: Duration = Duration::from_millis(5);
const EXPECTED_SCRIPTED_POLLS: usize = 4;
const NO_SIGNALS: usize = 0;

#[test]
fn unused_reservation_releases_its_reserved_thread() {
    let thread_exit = Arc::new(AtomicBool::new(false));
    let reservation =
        StartupUntrackedReaperReservation::reserve_observed(Arc::clone(&thread_exit)).unwrap();

    drop(reservation);

    wait_until(|| thread_exit.load(Ordering::Acquire));
}

#[test]
fn exclusive_handoff_retains_child_and_output_until_settlement() {
    let output_progressed = Arc::new(AtomicUsize::new(usize::default()));
    let output = StartupOutputProgress::Exclusive(ExclusivePtyOutput::observed(Arc::clone(
        &output_progressed,
    )));
    assert_handoff_retains_custody(output, &output_progressed);
}

#[test]
fn reader_backed_handoff_retains_child_and_output_until_settlement() {
    let output_progressed = Arc::new(AtomicUsize::new(usize::default()));
    let output = StartupOutputProgress::ReaderBacked(ReaderBackedPtyOutput::observed(Arc::clone(
        &output_progressed,
    )));
    assert_handoff_retains_custody(output, &output_progressed);
}

#[test]
fn reaped_child_waits_for_reader_backed_transcript_completion() {
    let observation = ChildObservation::new();
    let output_progressed = Arc::new(AtomicUsize::new(usize::default()));
    let lifecycle = Arc::new(crate::worker::ReaderLifecycle::default());
    let output = ReaderBackedPtyOutput::observed_with_lifecycle(
        Arc::clone(&output_progressed),
        Arc::clone(&lifecycle),
    );
    let thread_exit = Arc::new(AtomicBool::new(false));
    let reservation =
        StartupUntrackedReaperReservation::reserve_observed(Arc::clone(&thread_exit)).unwrap();
    let cleanup = StartupUntrackedCleanup::closed_gate_for_test(
        Box::new(GatedChild::observed(&observation)),
        StartupOutputProgress::ReaderBacked(output),
    );
    reservation.handoff(cleanup);
    wait_until(|| output_progressed.load(Ordering::Acquire) > usize::default());
    let polls_before_exit = observation.polls.load(Ordering::Acquire);

    observation.settled.store(true, Ordering::Release);
    wait_until(|| observation.polls.load(Ordering::Acquire) > polls_before_exit);
    assert!(!thread_exit.load(Ordering::Acquire));
    assert!(!observation.dropped.load(Ordering::Acquire));
    lifecycle.mark_drained();
    wait_until(|| thread_exit.load(Ordering::Acquire));
    assert!(observation.dropped.load(Ordering::Acquire));
}

#[test]
fn callback_errors_and_panics_are_retried_without_numeric_kill() {
    let observation = ChildObservation::new();
    let output_progressed = Arc::new(AtomicUsize::new(usize::default()));
    let thread_exit = Arc::new(AtomicBool::new(false));
    let reservation =
        StartupUntrackedReaperReservation::reserve_observed(Arc::clone(&thread_exit)).unwrap();
    let cleanup = StartupUntrackedCleanup::closed_gate_for_test(
        Box::new(ScriptedChild::observed(&observation)),
        StartupOutputProgress::Exclusive(ExclusivePtyOutput::observed(Arc::clone(
            &output_progressed,
        ))),
    );

    reservation.handoff(cleanup);

    wait_until(|| thread_exit.load(Ordering::Acquire));
    assert!(observation.dropped.load(Ordering::Acquire));
    assert!(output_progressed.load(Ordering::Acquire) > usize::default());
    assert_eq!(
        observation.polls.load(Ordering::Acquire),
        EXPECTED_SCRIPTED_POLLS
    );
    assert_eq!(observation.signals.load(Ordering::Acquire), NO_SIGNALS);
}

#[test]
fn exited_child_without_closed_gate_cannot_complete_output() {
    let observation = ChildObservation::new();
    observation.settled.store(true, Ordering::Release);
    let output_progressed = Arc::new(AtomicUsize::new(usize::default()));
    let mut cleanup = StartupUntrackedCleanup::missing_gate_for_test(
        Box::new(GatedChild::observed(&observation)),
        StartupOutputProgress::Exclusive(ExclusivePtyOutput::observed(Arc::clone(
            &output_progressed,
        ))),
    );

    assert!(!cleanup.drive_once_for_test());
    assert!(output_progressed.load(Ordering::Acquire) > usize::default());
    assert_eq!(observation.signals.load(Ordering::Acquire), NO_SIGNALS);
}

#[test]
fn lost_wait_authority_without_live_identity_cannot_use_gate() {
    let observation = ChildObservation::new();
    observation.settled.store(true, Ordering::Release);
    let output_progressed = Arc::new(AtomicUsize::new(usize::default()));
    let mut cleanup = StartupUntrackedCleanup::lost_wait_authority_for_test(
        Box::new(GatedChild::observed(&observation)),
        StartupOutputProgress::Exclusive(ExclusivePtyOutput::observed(Arc::clone(
            &output_progressed,
        ))),
    );

    assert!(!cleanup.drive_once_for_test());
    assert!(output_progressed.load(Ordering::Acquire) > usize::default());
    assert_eq!(observation.polls.load(Ordering::Acquire), usize::default());
    assert_eq!(observation.signals.load(Ordering::Acquire), NO_SIGNALS);
}

fn assert_handoff_retains_custody(output: StartupOutputProgress, output_progressed: &AtomicUsize) {
    let observation = ChildObservation::new();
    let thread_exit = Arc::new(AtomicBool::new(false));
    let reservation =
        StartupUntrackedReaperReservation::reserve_observed(Arc::clone(&thread_exit)).unwrap();
    let cleanup = StartupUntrackedCleanup::closed_gate_for_test(
        Box::new(GatedChild::observed(&observation)),
        output,
    );

    reservation.handoff(cleanup);

    wait_until(|| {
        output_progressed.load(Ordering::Acquire) > usize::default()
            && observation.polls.load(Ordering::Acquire) > usize::default()
    });
    assert!(!observation.dropped.load(Ordering::Acquire));
    assert_eq!(observation.signals.load(Ordering::Acquire), NO_SIGNALS);
    observation.settled.store(true, Ordering::Release);
    wait_until(|| thread_exit.load(Ordering::Acquire));
    assert!(observation.dropped.load(Ordering::Acquire));
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + TEST_SETTLEMENT_TIMEOUT;
    while !predicate() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(TEST_WAIT_INTERVAL);
    }
}

#[path = "startup_untracked_reaper_live_identity_tests.rs"]
mod live_identity_tests;
#[path = "startup_untracked_missing_eof_tests.rs"]
mod missing_eof_tests;
#[path = "startup_untracked_reaper_retry_tests.rs"]
mod retry_tests;
