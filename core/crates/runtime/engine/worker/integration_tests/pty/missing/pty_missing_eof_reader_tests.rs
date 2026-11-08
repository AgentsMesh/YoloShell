use super::*;
use crate::worker::ReaderLifecycle;
use crate::worker::reader_lifecycle::PendingOutput;

const PENDING_OUTPUT: &[u8] = b"missing-eof-pending-output";

fn expired() -> MissingEofExpired {
    MissingEofDeadline::with_timeout(Duration::ZERO)
        .observe(OutputProgressState::Quiescent, Instant::now())
        .unwrap()
        .unwrap()
}

#[test]
fn pending_reader_output_survives_expired_deadline_recheck() {
    let lifecycle = ReaderLifecycle::default();
    lifecycle.progress.lock().unwrap().pending = vec![PendingOutput {
        data: PENDING_OUTPUT.to_vec(),
        attempt: None,
    }];

    assert!(matches!(
        lifecycle.settle_missing_eof(expired()),
        OutputAbandonment::Retry
    ));
    assert!(lifecycle.has_pending());
    assert!(!lifecycle.is_cancelled());
}

#[test]
fn in_flight_read_blocks_expired_deadline_cancellation() {
    let lifecycle = ReaderLifecycle::default();
    let read = lifecycle.begin_output_read().unwrap();

    assert!(matches!(
        lifecycle.settle_missing_eof(expired()),
        OutputAbandonment::Retry
    ));
    assert!(!lifecycle.is_cancelled());
    drop(read);
}

#[test]
fn eof_publication_window_blocks_expired_deadline_cancellation() {
    let lifecycle = ReaderLifecycle::default();
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();

    assert!(matches!(
        lifecycle.settle_missing_eof(expired()),
        OutputAbandonment::Retry
    ));
    assert!(!lifecycle.is_cancelled());
    publication.publish();
    assert!(matches!(
        lifecycle.settle_missing_eof(expired()),
        OutputAbandonment::Retry
    ));
}

#[test]
fn quiescent_reader_is_cancelled_only_after_atomic_recheck() {
    let lifecycle = ReaderLifecycle::default();
    assert!(lifecycle.begin_attempt());

    assert!(matches!(
        lifecycle.settle_missing_eof(expired()),
        OutputAbandonment::Abandoned(_)
    ));
    assert!(lifecycle.is_cancelled());
    assert!(lifecycle.progress.lock().unwrap().read_stopped);
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Abandoned
    );
    assert!(lifecycle.begin_output_read().is_err());
    lifecycle.finish_attempt();
}

#[test]
fn late_eof_wins_over_expired_deadline() {
    let lifecycle = ReaderLifecycle::default();
    lifecycle.mark_eof_observed();

    assert!(matches!(
        lifecycle.settle_missing_eof(expired()),
        OutputAbandonment::Retry
    ));
    assert!(!lifecycle.is_cancelled());
}
