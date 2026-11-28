use super::support::{ChildObservation, GatedChild};
use super::*;

const INJECTED_OUTER_CLEANUP_PANIC: &str = "injected outer startup cleanup panic";
const EXPECTED_CLEANUP_ATTEMPTS: usize = 2;
const NO_SIGNALS: usize = 0;

#[test]
fn outer_unwind_retains_child_output_and_gate_for_retry() {
    let observation = ChildObservation::new();
    observation.settled.store(true, Ordering::Release);
    let output_progressed = Arc::new(AtomicUsize::default());
    let thread_exit = Arc::new(AtomicBool::default());
    let attempts = Arc::new(AtomicUsize::default());
    let observed_attempts = Arc::clone(&attempts);
    let reservation =
        StartupUntrackedReaperReservation::reserve_observed(Arc::clone(&thread_exit)).unwrap();
    let mut cleanup = StartupUntrackedCleanup::closed_gate_for_test(
        Box::new(GatedChild::observed(&observation)),
        StartupOutputProgress::Exclusive(ExclusivePtyOutput::observed(Arc::clone(
            &output_progressed,
        ))),
    );
    let mut panic_once = true;

    reservation.handoff_task(move || {
        observed_attempts.fetch_add(1, Ordering::AcqRel);
        if std::mem::take(&mut panic_once) {
            panic!("{INJECTED_OUTER_CLEANUP_PANIC}");
        }
        cleanup.reap();
    });

    wait_until(|| thread_exit.load(Ordering::Acquire));
    assert_eq!(attempts.load(Ordering::Acquire), EXPECTED_CLEANUP_ATTEMPTS);
    assert!(observation.dropped.load(Ordering::Acquire));
    assert!(observation.polls.load(Ordering::Acquire) > usize::default());
    assert!(output_progressed.load(Ordering::Acquire) > usize::default());
    assert_eq!(observation.signals.load(Ordering::Acquire), NO_SIGNALS);
}
