use super::*;
use crate::worker::ReaderLifecycle;

#[test]
fn reported_gone_active_reader_is_abandoned_without_signaling_hidden_holder() {
    let observation = ChildObservation::new();
    observation.settled.store(true, Ordering::Release);
    let lifecycle = Arc::new(ReaderLifecycle::default());
    assert!(lifecycle.begin_attempt());
    let output_calls = Arc::new(AtomicUsize::default());
    let output = ReaderBackedPtyOutput::observed_with_lifecycle(
        Arc::clone(&output_calls),
        Arc::clone(&lifecycle),
    );
    let mut cleanup = StartupUntrackedCleanup::closed_gate_for_test(
        Box::new(GatedChild::observed(&observation)),
        StartupOutputProgress::ReaderBacked(output),
    );
    cleanup.set_missing_eof_timeout_for_test(Duration::ZERO);

    assert!(!cleanup.drive_once_for_test());
    assert!(lifecycle.producers_are_gone());
    assert!(cleanup.drive_once_for_test());
    assert!(lifecycle.is_cancelled());
    assert_eq!(observation.signals.load(Ordering::Acquire), NO_SIGNALS);
    lifecycle.finish_attempt();
}

#[test]
fn reported_gone_exclusive_output_is_abandoned_without_signaling_hidden_holder() {
    let observation = ChildObservation::new();
    observation.settled.store(true, Ordering::Release);
    let output_calls = Arc::new(AtomicUsize::default());
    let output = ExclusivePtyOutput::quiescent_for_test(Arc::clone(&output_calls));
    let mut cleanup = StartupUntrackedCleanup::closed_gate_for_test(
        Box::new(GatedChild::observed(&observation)),
        StartupOutputProgress::Exclusive(output),
    );
    cleanup.set_missing_eof_timeout_for_test(Duration::ZERO);

    assert!(!cleanup.drive_once_for_test());
    assert!(cleanup.drive_once_for_test());
    assert!(output_calls.load(Ordering::Acquire) > usize::default());
    assert_eq!(observation.signals.load(Ordering::Acquire), NO_SIGNALS);
}
