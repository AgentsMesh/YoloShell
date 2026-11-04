use super::*;
use crate::worker::pty_output_progress::{ExclusivePtyOutput, ReaderBackedPtyOutput};
use crate::worker::{ReaderLifecycle, UnreleasedBootstrapGate};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

const INJECTED_PROGRESS_ERROR: &str = "injected untracked output error";
const INJECTED_PROGRESS_PANIC: &str = "injected untracked output panic";
const INJECTED_PAYLOAD_DROP_PANIC: &str = "injected untracked payload Drop panic";

struct PanicOnDropPayload;

impl Drop for PanicOnDropPayload {
    fn drop(&mut self) {
        panic!("{INJECTED_PAYLOAD_DROP_PANIC}");
    }
}

#[test]
fn containment_classifies_every_progress_outcome() {
    for state in [
        OutputProgressState::Complete,
        OutputProgressState::Abandoned,
    ] {
        assert!(contain_progress(&mut None, || Ok(state)));
    }
    for state in [OutputProgressState::Pending, OutputProgressState::Quiescent] {
        assert!(!contain_progress(&mut None, || Ok(state)));
    }

    let mut failure = None;
    assert!(!contain_progress(&mut failure, || {
        Err(anyhow::anyhow!(INJECTED_PROGRESS_ERROR))
    }));
    assert!(
        failure
            .unwrap()
            .to_string()
            .contains(INJECTED_PROGRESS_ERROR)
    );

    let mut failure = None;
    assert!(!contain_progress(&mut failure, || {
        panic!("{INJECTED_PROGRESS_PANIC}")
    }));
    assert!(
        failure
            .unwrap()
            .to_string()
            .contains(UNTRACKED_PROGRESS_PANIC_DETAIL)
    );
}

#[test]
fn progress_containment_consumes_a_panicking_payload_without_unwinding() {
    let mut failure = None;
    let completed = contain_progress(&mut failure, || std::panic::panic_any(PanicOnDropPayload));

    assert!(!completed);
    assert!(
        failure
            .unwrap()
            .to_string()
            .contains(UNTRACKED_PROGRESS_PANIC_DETAIL)
    );
}

#[test]
fn output_progress_and_completion_require_matching_reader_custody() {
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    let mut exclusive =
        StartupOutputProgress::Exclusive(ExclusivePtyOutput::observed(Arc::clone(&observed)));
    assert_eq!(
        progress_output(&mut exclusive, &mut None).unwrap(),
        OutputProgressState::Complete
    );
    assert_eq!(
        complete_output(&mut exclusive, &mut None, proof()).unwrap(),
        OutputProgressState::Complete
    );

    let output = ReaderBackedPtyOutput::observed(Arc::clone(&observed));
    let mut recovery = Some(output.recovery());
    let mut reader_backed = StartupOutputProgress::ReaderBacked(output);
    assert_eq!(
        progress_output(&mut reader_backed, &mut recovery).unwrap(),
        OutputProgressState::Complete
    );
    assert_eq!(
        complete_output(&mut reader_backed, &mut recovery, proof()).unwrap(),
        OutputProgressState::Complete
    );
    assert!(progress_output(&mut reader_backed, &mut None).is_err());
    assert!(complete_output(&mut reader_backed, &mut None, proof()).is_err());
}

#[test]
fn missing_eof_containment_covers_retry_abandonment_and_missing_custody() {
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    let mut complete =
        StartupOutputProgress::Exclusive(ExclusivePtyOutput::observed(Arc::clone(&observed)));
    assert!(!contain_missing_eof(
        &mut None,
        &mut deadline(),
        &mut complete,
        &mut None,
    ));

    let mut quiescent = StartupOutputProgress::Exclusive(ExclusivePtyOutput::quiescent_for_test(
        Arc::clone(&observed),
    ));
    let mut failure = None;
    assert!(contain_missing_eof(
        &mut failure,
        &mut deadline(),
        &mut quiescent,
        &mut None,
    ));
    assert!(
        failure
            .unwrap()
            .to_string()
            .contains(UNTRACKED_OUTPUT_ABANDONED_DETAIL)
    );

    let lifecycle = Arc::new(ReaderLifecycle::default());
    let output = ReaderBackedPtyOutput::observed_with_lifecycle(observed, lifecycle);
    let mut recovery = Some(output.recovery());
    let mut reader_backed = StartupOutputProgress::ReaderBacked(output);
    assert!(contain_missing_eof(
        &mut None,
        &mut deadline(),
        &mut reader_backed,
        &mut recovery,
    ));

    let mut failure = None;
    assert!(!contain_missing_eof(
        &mut failure,
        &mut deadline(),
        &mut reader_backed,
        &mut None,
    ));
    assert!(failure.is_some());
}

fn proof() -> UntrackedStartupProducersGone {
    UntrackedStartupProducersGone::from_exited_closed_gate(UnreleasedBootstrapGate)
}

fn deadline() -> MissingEofDeadline {
    MissingEofDeadline::with_timeout(Duration::ZERO)
}
