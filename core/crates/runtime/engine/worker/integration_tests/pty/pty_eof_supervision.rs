use crate::worker::tests::*;
const EOF_SUPERVISION_REOPEN_LIMIT: u32 = 2;
const EOF_SUPERVISION_FAILURE_RESTART_LIMIT: u32 = 1;
const EOF_SUPERVISION_EXIT_CODE: i32 = 37;
const EOF_SUPERVISION_TIMEOUT: Duration = Duration::from_secs(2);
const EOF_SUPERVISION_FINAL_DRAIN_ATTEMPTS: usize = 1;
const EOF_SUPERVISION_FINAL_TAIL: &[u8] = b"final-pty-tail";
const EOF_PROGRESS_RESET_REOPEN_LIMIT: u32 = 1;
const EOF_PROGRESS_RESET_EXPECTED_ATTEMPTS: usize = 3;
const EOF_PROGRESS_RESET_EXPECTED_RESTARTS: usize = 2;
const EOF_PROGRESS_FIRST_FAILURE_COUNT: u32 = 1;
const EOF_PROGRESS_SCRIPT_EXHAUSTED: &str = "EOF supervision test script exhausted";

fn expected_reopen_attempts(limit: u32) -> usize {
    usize::try_from(limit).unwrap()
}

fn expected_unconfirmed_attempts(limit: u32) -> usize {
    expected_reopen_attempts(limit).saturating_add(EOF_SUPERVISION_FINAL_DRAIN_ATTEMPTS)
}

#[test]
fn exhausted_unconfirmed_eof_waits_for_exit_then_drains_tail() {
    let fixture = Fixture::new();
    let expected_unconfirmed = expected_unconfirmed_attempts(EOF_SUPERVISION_REOPEN_LIMIT);
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::default());
    let restarts = Arc::new(std::sync::atomic::AtomicUsize::default());
    let (exhausted_tx, exhausted_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (supervisor_done_tx, supervisor_done_rx) = mpsc::channel();
    let supervisor_state = Arc::clone(&fixture.state);
    let attempt_counter = Arc::clone(&attempts);
    let restart_counter = Arc::clone(&restarts);
    let supervisor = std::thread::spawn(move || {
        supervise_reader_with(
            Box::new(io::empty()),
            supervisor_state,
            EOF_SUPERVISION_REOPEN_LIMIT,
            EOF_SUPERVISION_FAILURE_RESTART_LIMIT,
            move |_reader, state| {
                let attempt_number = attempt_counter
                    .fetch_add(EOF_SUPERVISION_FINAL_DRAIN_ATTEMPTS, Ordering::AcqRel)
                    .saturating_add(EOF_SUPERVISION_FINAL_DRAIN_ATTEMPTS);
                let _attempt = ReaderAttemptGuard::start(&state.reader_lifecycle);
                if state.child_waited.load(Ordering::Acquire) {
                    commit_reader_output(&state, EOF_SUPERVISION_FINAL_TAIL)?;
                    state.reader_lifecycle.mark_eof_observed();
                    return Ok(ReaderLoopOutcome::ChildExited);
                }
                if attempt_number == expected_unconfirmed {
                    exhausted_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                }
                Ok(ReaderLoopOutcome::UnexpectedEof {
                    output_observed: false,
                })
            },
            move |_failure, _failure_count, _phase| {
                restart_counter.fetch_add(EOF_SUPERVISION_FINAL_DRAIN_ATTEMPTS, Ordering::AcqRel);
            },
        );
        supervisor_done_tx.send(()).unwrap();
    });

    exhausted_rx.recv_timeout(EOF_SUPERVISION_TIMEOUT).unwrap();
    let progress = lock_recover(&fixture.state.reader_lifecycle.progress);
    assert!(progress.active);
    assert!(!progress.eof.is_observed());
    drop(progress);
    assert!(!fixture.state.child_waited.load(Ordering::Acquire));
    assert_eq!(attempts.load(Ordering::Acquire), expected_unconfirmed);
    assert_eq!(
        restarts.load(Ordering::Acquire),
        expected_reopen_attempts(EOF_SUPERVISION_REOPEN_LIMIT)
    );

    release_tx.send(()).unwrap();
    let (finalizer_done_tx, finalizer_done_rx) = mpsc::channel();
    let finalizer_state = Arc::clone(&fixture.state);
    let finalizer = std::thread::spawn(move || {
        finalize_child_exit(
            &finalizer_state,
            Some(EOF_SUPERVISION_EXIT_CODE),
            None,
            Duration::ZERO,
        );
        finalizer_done_tx.send(()).unwrap();
    });
    supervisor_done_rx
        .recv_timeout(EOF_SUPERVISION_TIMEOUT)
        .unwrap();
    finalizer_done_rx
        .recv_timeout(EOF_SUPERVISION_TIMEOUT)
        .unwrap();
    supervisor.join().unwrap();
    finalizer.join().unwrap();

    assert_eq!(
        attempts.load(Ordering::Acquire),
        expected_unconfirmed.saturating_add(EOF_SUPERVISION_FINAL_DRAIN_ATTEMPTS)
    );
    let progress = lock_recover(&fixture.state.reader_lifecycle.progress);
    assert!(progress.is_drained());
    drop(progress);
    let chunks = fixture
        .state
        .journal
        .lock()
        .unwrap()
        .replay_after(u64::default(), usize::MAX)
        .unwrap();
    let output: Vec<u8> = chunks.into_iter().flat_map(|chunk| chunk.data).collect();
    assert_eq!(output, EOF_SUPERVISION_FINAL_TAIL);
    let exit = lock_recover(&fixture.state.exit).clone().unwrap();
    assert_eq!(exit.code, Some(EOF_SUPERVISION_EXIT_CODE));
    assert_eq!(
        exit.last_output_sequence,
        fixture.state.journal.lock().unwrap().last_sequence()
    );
}

#[test]
fn observed_output_resets_consecutive_eof_recovery_budget() {
    let fixture = Fixture::new();
    let mut outcomes = std::collections::VecDeque::from([
        ReaderLoopOutcome::UnexpectedEof {
            output_observed: false,
        },
        ReaderLoopOutcome::UnexpectedEof {
            output_observed: true,
        },
        ReaderLoopOutcome::ChildExited,
    ]);
    let mut attempts = usize::default();
    let mut failure_counts = Vec::new();
    supervise_reader_with(
        Box::new(io::empty()),
        Arc::clone(&fixture.state),
        EOF_PROGRESS_RESET_REOPEN_LIMIT,
        EOF_SUPERVISION_FAILURE_RESTART_LIMIT,
        |_reader, state| {
            attempts = attempts.saturating_add(EOF_SUPERVISION_FINAL_DRAIN_ATTEMPTS);
            let outcome = outcomes.pop_front().expect(EOF_PROGRESS_SCRIPT_EXHAUSTED);
            let _attempt = ReaderAttemptGuard::start(&state.reader_lifecycle);
            if outcome == ReaderLoopOutcome::ChildExited {
                state.child_waited.store(true, Ordering::Release);
                state.reader_lifecycle.mark_eof_observed();
            }
            Ok(outcome)
        },
        |_failure, failure_count, _phase| failure_counts.push(failure_count),
    );

    assert_eq!(attempts, EOF_PROGRESS_RESET_EXPECTED_ATTEMPTS);
    assert!(outcomes.is_empty());
    assert_eq!(failure_counts.len(), EOF_PROGRESS_RESET_EXPECTED_RESTARTS);
    assert!(
        failure_counts
            .iter()
            .all(|count| *count == EOF_PROGRESS_FIRST_FAILURE_COUNT)
    );
    assert!(lock_recover(&fixture.state.reader_lifecycle.progress).is_drained());
}

#[test]
fn cancellation_releases_unconfirmed_exit_wait_without_fake_eof() {
    let fixture = Fixture::new();
    let (attempted_tx, attempted_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let state = Arc::clone(&fixture.state);
    let supervisor = std::thread::spawn(move || {
        supervise_reader_with(
            Box::new(io::empty()),
            state,
            u32::default(),
            EOF_SUPERVISION_FAILURE_RESTART_LIMIT,
            move |_reader, _state| {
                attempted_tx.send(()).unwrap();
                Ok(ReaderLoopOutcome::UnexpectedEof {
                    output_observed: false,
                })
            },
            |_failure, _failure_count, _phase| {},
        );
        done_tx.send(()).unwrap();
    });

    attempted_rx.recv_timeout(EOF_SUPERVISION_TIMEOUT).unwrap();
    fixture.state.reader_lifecycle.cancel();
    done_rx.recv_timeout(EOF_SUPERVISION_TIMEOUT).unwrap();
    supervisor.join().unwrap();
    let progress = lock_recover(&fixture.state.reader_lifecycle.progress);
    assert!(!fixture.state.child_waited.load(Ordering::Acquire));
    assert!(!progress.eof.is_observed());
    assert!(progress.read_stopped);
    assert!(progress.is_drained());
}
