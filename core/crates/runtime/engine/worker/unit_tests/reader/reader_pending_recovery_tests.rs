const READER_PENDING_FAILURE_DETAIL: &str = "injected failure after staging PTY output";
const READER_PENDING_OUTPUT: &[u8] = b"pending-final-drain-tail";
const READER_PENDING_ATTEMPT_INCREMENT: usize = 1;
const READER_PENDING_POST_BUDGET_ATTEMPTS: usize = 2;
const READER_PENDING_EXPECTED_CHUNKS: usize = 1;

#[test]
fn exhausted_final_drain_commits_pending_output_before_stopping() {
    let fixture = Fixture::new();
    fixture.state.child_waited.store(true, Ordering::Release);
    fixture
        .state
        .reader_lifecycle
        .notify_child_exit_confirmation();
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::default());
    let restarts = Arc::new(Mutex::new(Vec::new()));
    let pending_attempt = usize::try_from(PTY_READER_FAILURE_RESTART_LIMIT).unwrap();
    let completed_attempts = pending_attempt.saturating_add(READER_PENDING_POST_BUDGET_ATTEMPTS);
    let attempt_count = Arc::clone(&attempts);
    let restart_records = Arc::clone(&restarts);

    supervise_reader_with(
        Box::new(io::empty()),
        Arc::clone(&fixture.state),
        PTY_READER_UNCONFIRMED_EOF_REOPEN_LIMIT,
        PTY_READER_FAILURE_RESTART_LIMIT,
        move |_reader, state| {
            let attempt =
                attempt_count.fetch_add(READER_PENDING_ATTEMPT_INCREMENT, Ordering::AcqRel);
            if attempt < pending_attempt {
                return Err(anyhow::anyhow!(READER_PENDING_FAILURE_DETAIL));
            }
            if attempt == pending_attempt {
                stage_reader_output(&state, READER_PENDING_OUTPUT)?;
                return Err(anyhow::anyhow!(READER_PENDING_FAILURE_DETAIL));
            }
            state.reader_lifecycle.mark_eof_observed();
            Ok(ReaderLoopOutcome::ChildExited)
        },
        move |_failure, count, phase| restart_records.lock().unwrap().push((count, phase)),
    );

    assert_eq!(attempts.load(Ordering::Acquire), completed_attempts);
    let records = restarts.lock().unwrap();
    assert_eq!(records.len(), pending_attempt);
    assert_eq!(
        records.last().copied(),
        Some((
            PTY_READER_FAILURE_RESTART_LIMIT,
            ReaderRestartPhase::FinalDrain
        ))
    );
    drop(records);
    let chunks = fixture
        .state
        .journal
        .lock()
        .unwrap()
        .replay_after(u64::default(), usize::MAX)
        .unwrap();
    assert_eq!(chunks.len(), READER_PENDING_EXPECTED_CHUNKS);
    assert_eq!(chunks[usize::default()].data, READER_PENDING_OUTPUT);
    assert!(!fixture.state.reader_lifecycle.has_pending());
    assert_eq!(fixture.state.termination_reason(), None);
    assert!(lock_recover(&fixture.state.reader_lifecycle.progress).is_drained());
}
