use crate::worker::pty_output_progress::OutputQuiescenceState;

#[test]
fn tentative_eof_timeout_retains_authority_without_publishing_completion() {
    let lifecycle = ReaderLifecycle::default();
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();
    assert!(lifecycle.kernel_eof_identity().is_some());
    let epoch = lifecycle.request_output_probe().unwrap();

    assert_eq!(
        lifecycle.output_quiescence_state(epoch),
        OutputQuiescenceState::Reached
    );
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Pending
    );
    let progress = lifecycle.lock_progress();
    assert!(progress.eof.is_awaiting_publication());
    assert!(progress.output_read_in_flight);
    assert!(!progress.eof.is_observed());
    drop(progress);
    drop(publication);
    let progress = lifecycle.lock_progress();
    assert!(progress.eof.is_retained());
    assert!(!progress.output_read_in_flight);
    assert!(!progress.eof.is_observed());
    assert!(progress.eof.has_authority());
}

#[test]
fn dropping_tentative_eof_retains_authority_and_restores_reader_admission() {
    let lifecycle = ReaderLifecycle::default();
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();

    drop(publication);

    let progress = lifecycle.lock_progress();
    assert!(!progress.eof.is_awaiting_publication());
    assert!(progress.eof.is_retained());
    assert!(!progress.output_read_in_flight);
    assert!(!progress.eof.is_observed());
    drop(progress);
    assert!(lifecycle.begin_output_read().is_ok());
}

#[test]
fn retained_eof_cannot_authorize_while_a_read_is_in_flight() {
    let lifecycle = ReaderLifecycle::default();
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();
    drop(publication);
    let identity = lifecycle.kernel_eof_identity().unwrap();
    let epoch = lifecycle.request_output_probe().unwrap();
    let read = lifecycle.begin_output_read().unwrap();

    assert_eq!(
        lifecycle.output_quiescence_state(epoch),
        OutputQuiescenceState::Pending
    );
    assert!(
        !lifecycle
            .validates_current_kernel_eof(&identity, || Ok(true))
            .unwrap()
    );
    assert!(lifecycle.kernel_eof_identity().is_none());

    read.mark_quiescent().unwrap();
    assert!(lifecycle.kernel_eof_identity().is_none());
}

#[test]
fn retained_eof_is_replaced_after_a_would_block_challenge() {
    let lifecycle = ReaderLifecycle::default();
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();
    drop(publication);
    let identity = lifecycle.kernel_eof_identity().unwrap();

    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();

    assert!(lifecycle.kernel_eof_identity().is_none());
    let current = lifecycle
        .acquire_kernel_eof_probe(|| Ok(true))
        .unwrap()
        .unwrap();
    assert!(!std::sync::Arc::ptr_eq(&identity, &current));
}

#[test]
fn publishing_tentative_eof_completes_output_atomically() {
    let lifecycle = ReaderLifecycle::default();
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();

    publication.publish();
    assert!(lifecycle.kernel_eof_identity().is_some());

    let progress = lifecycle.lock_progress();
    assert!(!progress.eof.is_awaiting_publication());
    assert!(!progress.output_read_in_flight);
    assert!(progress.eof.is_observed());
    drop(progress);
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Complete
    );
    lifecycle.cancel();
    assert!(lifecycle.kernel_eof_identity().is_none());
}

#[test]
fn pending_journal_output_overrides_tentative_eof_quiescence() {
    let lifecycle = ReaderLifecycle::default();
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();
    let epoch = lifecycle.request_output_probe().unwrap();
    lifecycle.lock_progress().pending = vec![PendingOutput {
        data: PENDING_TEST_OUTPUT.to_vec(),
        attempt: None,
    }];

    assert_eq!(
        lifecycle.output_quiescence_state(epoch),
        OutputQuiescenceState::Pending
    );
    lifecycle.clear_pending_output_for_test();
    drop(publication);
}

#[test]
fn stopped_reader_revokes_tentative_eof_and_blocks_late_publication() {
    let lifecycle = ReaderLifecycle::default();
    let publication = lifecycle
        .begin_output_read()
        .unwrap()
        .kernel_eof_for_test()
        .await_eof_publication();

    lifecycle.mark_read_stopped();
    assert!(lifecycle.kernel_eof_identity().is_none());
    publication.publish();

    let progress = lifecycle.lock_progress();
    assert!(progress.read_stopped);
    assert!(!progress.eof.is_observed());
    assert!(!progress.eof.is_awaiting_publication());
    assert!(!progress.eof.has_authority());
    assert!(!progress.output_read_in_flight);
}

#[test]
fn requested_output_probe_releases_an_exhausted_reader_wait() {
    let lifecycle = std::sync::Arc::new(ReaderLifecycle::default());
    let confirmed = std::sync::Arc::new(std::sync::atomic::AtomicBool::default());
    let waiting_lifecycle = std::sync::Arc::clone(&lifecycle);
    let waiting_confirmation = std::sync::Arc::clone(&confirmed);
    let waiter = std::thread::spawn(move || {
        waiting_lifecycle.wait_for_child_exit_confirmation(&waiting_confirmation)
    });

    let epoch = lifecycle.request_output_probe().unwrap();

    assert_eq!(waiter.join().unwrap(), ReaderExitConfirmation::OutputProbe);
    assert!(lifecycle.output_reader_wakeup.load(Ordering::Acquire));
    let read = lifecycle.begin_output_read().unwrap();
    assert!(!lifecycle.output_reader_wakeup.load(Ordering::Acquire));
    read.mark_quiescent().unwrap();
    assert_eq!(
        lifecycle.output_quiescence_state(epoch),
        OutputQuiescenceState::Reached
    );
}
