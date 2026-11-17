use super::*;

const PENDING_TEST_OUTPUT: &[u8] = b"pending-output";
const ADDITIONAL_TEST_OUTPUT: &[u8] = b"additional-output";
const EXPECTED_PENDING_CHUNKS: usize = 2;

#[test]
fn pending_delivery_prevents_output_completion() {
    let lifecycle = ReaderLifecycle::default();
    {
        let mut progress = lifecycle.lock_progress();
        progress.eof.mark_drained();
        progress.pending = vec![PendingOutput {
            data: PENDING_TEST_OUTPUT.to_vec(),
            attempt: None,
        }];
    }

    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Pending
    );

    lifecycle.mark_drained();
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Complete
    );
}

#[test]
fn probe_epoch_exhaustion_fails_without_wrapping() {
    let lifecycle = ReaderLifecycle::default();
    lifecycle.exhaust_output_probe_epoch_for_test();

    assert!(lifecycle.request_output_probe().is_err());
    let progress = lifecycle.lock_progress();
    assert_eq!(
        progress.requested_probe_epoch,
        OutputProbeEpoch::maximum_for_test()
    );
    assert!(progress.drained_probe_epoch.is_none());
    assert!(!lifecycle.output_reader_wakeup.load(Ordering::Acquire));
}

#[test]
fn eof_with_pending_output_does_not_reach_probe() {
    let lifecycle = ReaderLifecycle::default();
    let epoch = lifecycle.request_output_probe().unwrap();
    let mut progress = lifecycle.lock_progress();
    progress.eof.mark_drained();
    progress.pending = vec![PendingOutput {
        data: PENDING_TEST_OUTPUT.to_vec(),
        attempt: None,
    }];
    drop(progress);

    assert_eq!(
        lifecycle.output_quiescence_state(epoch),
        crate::worker::pty_output_progress::OutputQuiescenceState::Pending
    );
}

#[test]
fn pending_output_rejects_false_quiescence_publication() {
    let lifecycle = ReaderLifecycle::default();
    let epoch = lifecycle.request_output_probe().unwrap();
    let output_read = lifecycle.begin_output_read().unwrap();
    lifecycle.lock_progress().pending = vec![PendingOutput {
        data: PENDING_TEST_OUTPUT.to_vec(),
        attempt: None,
    }];

    assert!(output_read.mark_quiescent().is_err());
    lifecycle.clear_pending_output_for_test();
    assert_eq!(
        lifecycle.output_quiescence_state(epoch),
        crate::worker::pty_output_progress::OutputQuiescenceState::Pending
    );
    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();
    assert_eq!(
        lifecycle.output_quiescence_state(epoch),
        crate::worker::pty_output_progress::OutputQuiescenceState::Reached
    );
}

#[test]
fn dropped_read_attempt_clears_in_flight_state() {
    let lifecycle = ReaderLifecycle::default();
    let output_read = lifecycle.begin_output_read().unwrap();
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Pending
    );

    drop(output_read);

    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Quiescent
    );
}

#[test]
fn concurrent_output_read_attempt_is_rejected() {
    let lifecycle = ReaderLifecycle::default();
    let output_read = lifecycle.begin_output_read().unwrap();

    assert!(lifecycle.begin_output_read().is_err());
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Pending
    );
    drop(output_read);
}

#[test]
fn pending_delivery_allows_the_next_kernel_read_to_extend_the_batch() {
    let lifecycle = ReaderLifecycle::default();
    lifecycle.lock_progress().pending = vec![PendingOutput {
        data: PENDING_TEST_OUTPUT.to_vec(),
        attempt: None,
    }];

    let output_read = lifecycle.begin_output_read().unwrap();
    assert!(lifecycle.lock_progress().output_read_in_flight);
    drop(output_read);
    assert!(!lifecycle.lock_progress().output_read_in_flight);
}

#[test]
fn extending_a_pending_batch_revokes_an_old_drain_proof() {
    let lifecycle = ReaderLifecycle::default();
    let epoch = lifecycle.request_output_probe().unwrap();
    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();
    let output_read = lifecycle.begin_output_read().unwrap();
    lifecycle.lock_progress().pending = vec![PendingOutput {
        data: PENDING_TEST_OUTPUT.to_vec(),
        attempt: None,
    }];

    output_read.stage_for_test(ADDITIONAL_TEST_OUTPUT).unwrap();
    {
        let progress = lifecycle.lock_progress();
        assert_eq!(progress.pending.len(), EXPECTED_PENDING_CHUNKS);
        assert_eq!(progress.pending[0].data, PENDING_TEST_OUTPUT);
        assert_eq!(progress.pending[1].data, ADDITIONAL_TEST_OUTPUT);
        assert!(progress.drained_probe_epoch.is_none());
    }
    lifecycle.clear_pending_output_for_test();
    assert_eq!(
        lifecycle.output_quiescence_state(epoch),
        crate::worker::pty_output_progress::OutputQuiescenceState::Pending
    );
    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();
    assert_eq!(
        lifecycle.output_quiescence_state(epoch),
        crate::worker::pty_output_progress::OutputQuiescenceState::Reached
    );
}

#[test]
fn abandoned_read_does_not_publish_a_probe_epoch() {
    let lifecycle = ReaderLifecycle::default();
    let epoch = lifecycle.request_output_probe().unwrap();

    drop(lifecycle.begin_output_read().unwrap());

    assert_eq!(
        lifecycle.output_quiescence_state(epoch),
        crate::worker::pty_output_progress::OutputQuiescenceState::Pending
    );
}

include!("reader_lifecycle_tentative_eof_tests.rs");
include!("reader_lifecycle_branch_tests.rs");
include!("reader_lifecycle_eof_transaction_tests.rs");
