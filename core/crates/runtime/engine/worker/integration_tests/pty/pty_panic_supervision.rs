use crate::worker::tests::*;
const PANIC_SUPERVISION_FAILURE_RESTART_LIMIT: u32 = 2;
const PANIC_SUPERVISION_EOF_REOPEN_LIMIT: u32 = 1;
const PANIC_SUPERVISION_ATTEMPT_INCREMENT: usize = 1;
const PANIC_SUPERVISION_OBSERVATION_TIMEOUT: Duration = Duration::from_secs(2);
const PANIC_SUPERVISION_COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);
const PANIC_SUPERVISION_DISPLACED_HOME: &str = "panic-supervision-displaced-home";
const PANIC_SUPERVISION_DETAIL: &str = "injected persistent reader panic";
const PANIC_SUPERVISION_COMPLETION_DETAIL: &str =
    "reader supervisor did not settle after namespace loss";
const RECOVERABLE_DRAIN_FAILURE_DETAIL: &str = "injected recoverable final-drain panic";
const RECOVERABLE_DRAIN_FIRST_ATTEMPT: usize = 0;
const RECOVERABLE_DRAIN_EXPECTED_ATTEMPTS: usize = 2;
const RECOVERABLE_DRAIN_FIRST_RESTART: u32 = 1;

#[test]
fn repeated_panics_after_child_exit_require_typed_settlement() {
    let fixture = Fixture::new();
    fixture.state.child_waited.store(true, Ordering::Release);
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::default());
    let restarts = Arc::new(std::sync::atomic::AtomicUsize::default());
    let attempt_counter = Arc::clone(&attempts);
    let restart_counter = Arc::clone(&restarts);
    let expected_restarts = usize::try_from(PANIC_SUPERVISION_FAILURE_RESTART_LIMIT)
        .unwrap()
        .saturating_add(PANIC_SUPERVISION_ATTEMPT_INCREMENT);
    let (observed_tx, observed_rx) = mpsc::sync_channel(PANIC_SUPERVISION_ATTEMPT_INCREMENT);
    let (release_tx, release_rx) = mpsc::sync_channel(PANIC_SUPERVISION_ATTEMPT_INCREMENT);
    let (completion_tx, completion_rx) = mpsc::sync_channel(PANIC_SUPERVISION_ATTEMPT_INCREMENT);
    let supervisor_state = Arc::clone(&fixture.state);

    let supervisor = std::thread::spawn(move || {
        supervise_reader_with(
            Box::new(io::empty()),
            supervisor_state,
            PANIC_SUPERVISION_EOF_REOPEN_LIMIT,
            PANIC_SUPERVISION_FAILURE_RESTART_LIMIT,
            move |_reader, _state| -> Result<ReaderLoopOutcome> {
                attempt_counter.fetch_add(PANIC_SUPERVISION_ATTEMPT_INCREMENT, Ordering::AcqRel);
                panic!("{PANIC_SUPERVISION_DETAIL}");
            },
            move |_failure, _failure_count, _phase| {
                let observed = restart_counter
                    .fetch_add(PANIC_SUPERVISION_ATTEMPT_INCREMENT, Ordering::AcqRel)
                    .saturating_add(PANIC_SUPERVISION_ATTEMPT_INCREMENT);
                if observed == expected_restarts {
                    observed_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                }
            },
        );
        completion_tx.send(()).unwrap();
    });

    observed_rx
        .recv_timeout(PANIC_SUPERVISION_OBSERVATION_TIMEOUT)
        .unwrap();
    assert_eq!(attempts.load(Ordering::Acquire), expected_restarts);
    assert_eq!(restarts.load(Ordering::Acquire), expected_restarts);
    assert!(fixture.state.output_blocked());
    assert_eq!(fixture.state.termination_reason(), None);
    assert!(!lock_recover(&fixture.state.reader_lifecycle.progress).read_stopped);

    let displaced = fixture
        ._filesystem
        .scratch_path(PANIC_SUPERVISION_DISPLACED_HOME);
    std::fs::rename(fixture.state.paths.home(), displaced).unwrap();
    release_tx.send(()).unwrap();
    completion_rx
        .recv_timeout(PANIC_SUPERVISION_COMPLETION_TIMEOUT)
        .expect(PANIC_SUPERVISION_COMPLETION_DETAIL);
    supervisor.join().unwrap();
    assert_eq!(fixture.state.termination_reason(), Some(ExitReason::Lost));
    assert!(lock_recover(&fixture.state.reader_lifecycle.progress).read_stopped);
}

#[test]
fn first_final_drain_panic_after_namespace_loss_reopens_reader() {
    let fixture = Fixture::new();
    fixture.state.child_waited.store(true, Ordering::Release);
    let displaced = fixture
        ._filesystem
        .scratch_path(PANIC_SUPERVISION_DISPLACED_HOME);
    std::fs::rename(fixture.state.paths.home(), displaced).unwrap();
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::default());
    let restarts = Arc::new(Mutex::new(Vec::new()));
    let attempt_counter = Arc::clone(&attempts);
    let restart_records = Arc::clone(&restarts);

    supervise_reader_with(
        Box::new(io::empty()),
        Arc::clone(&fixture.state),
        PANIC_SUPERVISION_EOF_REOPEN_LIMIT,
        PANIC_SUPERVISION_FAILURE_RESTART_LIMIT,
        move |_reader, _state| {
            let attempt =
                attempt_counter.fetch_add(PANIC_SUPERVISION_ATTEMPT_INCREMENT, Ordering::AcqRel);
            if attempt == RECOVERABLE_DRAIN_FIRST_ATTEMPT {
                panic!("{RECOVERABLE_DRAIN_FAILURE_DETAIL}");
            }
            Ok(ReaderLoopOutcome::ChildExited)
        },
        move |_failure, count, phase| restart_records.lock().unwrap().push((count, phase)),
    );

    assert_eq!(
        attempts.load(Ordering::Acquire),
        RECOVERABLE_DRAIN_EXPECTED_ATTEMPTS
    );
    assert_eq!(
        *restarts.lock().unwrap(),
        [(
            RECOVERABLE_DRAIN_FIRST_RESTART,
            ReaderRestartPhase::FinalDrain
        )]
    );
    assert_eq!(fixture.state.termination_reason(), Some(ExitReason::Lost));
    assert!(fixture.state.output_blocked());
    assert!(fixture.state.reader_lifecycle.drains_after_namespace_loss());
    assert!(!lock_recover(&fixture.state.reader_lifecycle.progress).read_stopped);
}
