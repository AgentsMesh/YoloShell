const RESTART_PROBE_CAPACITY: usize = 1;

#[test]
fn stopped_reader_rejects_a_new_attempt_without_becoming_active() {
    let lifecycle = ReaderLifecycle::default();
    lifecycle.mark_read_stopped();

    assert!(!lifecycle.begin_attempt());
    assert!(!lifecycle.lock_progress().active);
}

#[test]
fn read_cancellation_interrupts_final_drain_before_wait_registration() {
    let lifecycle = ReaderLifecycle::default();
    let confirmed = AtomicBool::new(false);
    let (probe, observed) = std::sync::mpsc::sync_channel(RESTART_PROBE_CAPACITY);
    lifecycle.install_restart_wait_probe(probe);
    lifecycle.cancel();

    lifecycle.wait_for_restart(
        Duration::ZERO,
        &confirmed,
        ReaderRestartPhase::FinalDrain,
        ReaderRestartKind::Read,
    );

    assert_eq!(
        observed.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    );
}

#[test]
fn observed_eof_stays_quiescent_while_a_reader_attempt_is_active() {
    let lifecycle = ReaderLifecycle::default();
    assert!(lifecycle.begin_attempt());
    lifecycle.mark_eof_observed();

    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Quiescent
    );
    lifecycle.finish_attempt();
    assert_eq!(
        lifecycle.output_progress_state(),
        OutputProgressState::Complete
    );
}

#[test]
fn eof_commit_operation_runs_while_the_lifecycle_lock_is_held() {
    let lifecycle = ReaderLifecycle::default();
    lifecycle.mark_eof_observed();
    let identity = lifecycle.kernel_eof_identity().unwrap();
    let mut lock_was_held = false;

    let committed = lifecycle
        .with_validated_current_kernel_eof(
            &identity,
            || Ok(true),
            || {
                lock_was_held = matches!(
                    lifecycle.progress.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                );
                Ok(KernelEofTransactionDisposition::Preserve)
            },
        )
        .unwrap();

    assert!(committed);
    assert!(lock_was_held);
}
