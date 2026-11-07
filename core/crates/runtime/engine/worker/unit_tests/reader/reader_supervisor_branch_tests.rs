const SUPERVISOR_BRANCH_FAILURE: &str = "injected final-drain reader failure";
const SUPERVISOR_BRANCH_OUTPUT: &[u8] = b"retained-supervisor-output";
const SUPERVISOR_BRANCH_MOVED_SESSION: &str = "supervisor-branch-moved-session";
const SUPERVISOR_BRANCH_FIRST_ATTEMPT: usize = 1;
const SUPERVISOR_BRANCH_SECOND_ATTEMPT: usize = 2;

#[test]
fn final_drain_reopens_after_unexpected_eof_without_backoff() {
    let fixture = Fixture::new();
    fixture.state.child_waited.store(true, Ordering::Release);
    let attempts = std::cell::Cell::new(usize::default());
    let restarts = std::cell::Cell::new(usize::default());

    supervise_reader_with(
        Box::new(io::empty()),
        Arc::clone(&fixture.state),
        PTY_READER_UNCONFIRMED_EOF_REOPEN_LIMIT,
        PTY_READER_FAILURE_RESTART_LIMIT,
        |_, _| {
            let attempt = attempts.get() + SUPERVISOR_BRANCH_FIRST_ATTEMPT;
            attempts.set(attempt);
            if attempt == SUPERVISOR_BRANCH_FIRST_ATTEMPT {
                return Ok(ReaderLoopOutcome::UnexpectedEof {
                    output_observed: false,
                });
            }
            Ok(ReaderLoopOutcome::ChildExited)
        },
        |_, _, _| restarts.set(restarts.get() + SUPERVISOR_BRANCH_FIRST_ATTEMPT),
    );

    assert_eq!(attempts.get(), SUPERVISOR_BRANCH_SECOND_ATTEMPT);
    assert_eq!(restarts.get(), usize::default());
}

#[test]
fn exhausted_final_drain_failure_blocks_output_before_cancellation() {
    let fixture = Fixture::new();
    fixture.state.child_waited.store(true, Ordering::Release);
    let restart_state = Arc::clone(&fixture.state);
    let restarts = std::cell::Cell::new(usize::default());

    supervise_reader_with(
        Box::new(io::empty()),
        Arc::clone(&fixture.state),
        PTY_READER_UNCONFIRMED_EOF_REOPEN_LIMIT,
        u32::default(),
        |_, _| anyhow::bail!(SUPERVISOR_BRANCH_FAILURE),
        |_, _, phase| {
            assert_eq!(phase, ReaderRestartPhase::FinalDrain);
            restarts.set(restarts.get() + SUPERVISOR_BRANCH_FIRST_ATTEMPT);
            restart_state.reader_lifecycle.cancel();
        },
    );

    assert_eq!(restarts.get(), SUPERVISOR_BRANCH_FIRST_ATTEMPT);
    assert!(fixture.state.output_blocked());
    assert!(lock_recover(&fixture.state.reader_lifecycle.progress).read_stopped);
}

#[test]
fn retained_namespace_delivery_does_not_abandon_an_in_flight_read() {
    let fixture = Fixture::new();
    stage_reader_output(&fixture.state, SUPERVISOR_BRANCH_OUTPUT).unwrap();
    lock_recover(&fixture.state.reader_lifecycle.progress).output_read_in_flight = true;
    let session = fixture.state.paths.session_dir(fixture.terminal_id());
    let journal = fixture.state.paths.journal(fixture.terminal_id());
    let suffix = journal.strip_prefix(&session).unwrap().to_path_buf();
    let moved = fixture
        ._filesystem
        .scratch_path(SUPERVISOR_BRANCH_MOVED_SESSION);
    std::fs::rename(session, &moved).unwrap();
    let attempts = std::cell::Cell::new(usize::default());

    supervise_reader_with(
        Box::new(io::empty()),
        Arc::clone(&fixture.state),
        PTY_READER_UNCONFIRMED_EOF_REOPEN_LIMIT,
        PTY_READER_FAILURE_RESTART_LIMIT,
        |_, _| {
            attempts.set(attempts.get() + SUPERVISOR_BRANCH_FIRST_ATTEMPT);
            Ok(ReaderLoopOutcome::ChildExited)
        },
        |_, _, _| unreachable!(),
    );

    assert_eq!(attempts.get(), SUPERVISOR_BRANCH_FIRST_ATTEMPT);
    assert!(!fixture.state.reader_lifecycle.has_pending());
    assert!(lock_recover(&fixture.state.reader_lifecycle.progress).output_read_in_flight);
    assert_eq!(fixture.state.termination_reason(), None);
    let mut retained = Journal::open(moved.join(suffix), fixture.state.runtime_epoch()).unwrap();
    let chunks = retained.replay_after(u64::default(), usize::MAX).unwrap();
    assert_eq!(chunks.len(), SUPERVISOR_BRANCH_FIRST_ATTEMPT);
    assert_eq!(chunks[usize::default()].data, SUPERVISOR_BRANCH_OUTPUT);
}

#[test]
fn retained_namespace_delivery_drains_quiescent_output() {
    let fixture = Fixture::new();
    stage_reader_output(&fixture.state, SUPERVISOR_BRANCH_OUTPUT).unwrap();
    let session = fixture.state.paths.session_dir(fixture.terminal_id());
    let journal = fixture.state.paths.journal(fixture.terminal_id());
    let suffix = journal.strip_prefix(&session).unwrap().to_path_buf();
    let moved = fixture
        ._filesystem
        .scratch_path(SUPERVISOR_BRANCH_MOVED_SESSION);
    std::fs::rename(session, &moved).unwrap();
    let attempts = std::cell::Cell::new(usize::default());

    supervise_reader_with(
        Box::new(io::empty()),
        Arc::clone(&fixture.state),
        PTY_READER_UNCONFIRMED_EOF_REOPEN_LIMIT,
        PTY_READER_FAILURE_RESTART_LIMIT,
        |_, _| {
            attempts.set(attempts.get() + SUPERVISOR_BRANCH_FIRST_ATTEMPT);
            Ok(ReaderLoopOutcome::ChildExited)
        },
        |_, _, _| unreachable!(),
    );

    assert_eq!(attempts.get(), SUPERVISOR_BRANCH_FIRST_ATTEMPT);
    assert!(!fixture.state.reader_lifecycle.has_pending());
    assert!(fixture.state.reader_lifecycle.drains_after_namespace_loss());
    assert!(!fixture.state.reader_lifecycle.is_cancelled());
    assert!(fixture.state.output_blocked());
    assert_eq!(fixture.state.termination_reason(), Some(ExitReason::Lost));
    let mut retained = Journal::open(moved.join(suffix), fixture.state.runtime_epoch()).unwrap();
    let chunks = retained.replay_after(u64::default(), usize::MAX).unwrap();
    assert_eq!(chunks.len(), SUPERVISOR_BRANCH_FIRST_ATTEMPT);
    assert_eq!(chunks[usize::default()].data, SUPERVISOR_BRANCH_OUTPUT);
}

#[test]
fn reader_restart_without_a_master_reports_missing_authority() {
    let fixture = Fixture::new();
    *lock_recover(&fixture.state.master) = None;
    let mut custody = ReaderSupervisorCustody::new(
        Box::new(io::empty()),
        Arc::clone(&fixture.state.reader_lifecycle),
    );
    drop(next_reader(&mut custody, &fixture.state).unwrap());

    assert!(next_reader(&mut custody, &fixture.state).is_err());
}
