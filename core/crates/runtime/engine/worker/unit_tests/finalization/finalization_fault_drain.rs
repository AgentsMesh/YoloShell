use crate::worker::tests::*;
const READER_TRANSITION_TIMEOUT: Duration = Duration::from_secs(2);
const READER_BLOCKED_POLL_INTERVAL: Duration = Duration::from_millis(1);

fn spawn_blocked_finalizer(state: Arc<WorkerState>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        finalize_child_exit_scheduled(
            &state,
            None,
            None,
            FinalizationSchedule {
                drain_interval: Duration::ZERO,
                blocked_after: Duration::ZERO,
            },
        );
    })
}

fn wait_until_output_is_blocked(state: &WorkerState) {
    let deadline = Instant::now() + READER_TRANSITION_TIMEOUT;
    while !state.output_blocked() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(READER_BLOCKED_POLL_INTERVAL);
    }
}

#[test]
fn expired_finalization_threshold_does_not_cancel_or_upgrade_a_reader() {
    let fixture = Fixture::new();
    assert!(fixture.state.reader_lifecycle.begin_attempt());
    let finalizer = spawn_blocked_finalizer(Arc::clone(&fixture.state));

    wait_until_output_is_blocked(&fixture.state);
    assert!(!fixture.state.reader_lifecycle.is_cancelled());
    assert_eq!(fixture.state.termination_reason(), None);
    assert!(lock_recover(&fixture.state.exit).is_none());

    fixture.state.reader_lifecycle.mark_drained();
    finalizer.join().unwrap();
    let exit = lock_recover(&fixture.state.exit).clone().unwrap();
    assert_eq!(exit.reason, ExitReason::Natural);
    assert!(!fixture.state.output_blocked());
}

#[test]
fn lost_reason_without_output_proof_does_not_cancel_a_reader() {
    let fixture = Fixture::new();
    fixture.state.begin_termination_for_test(ExitReason::Lost);
    assert!(fixture.state.reader_lifecycle.begin_attempt());
    let finalizer = spawn_blocked_finalizer(Arc::clone(&fixture.state));

    wait_until_output_is_blocked(&fixture.state);
    assert!(!fixture.state.reader_lifecycle.is_cancelled());
    assert!(lock_recover(&fixture.state.exit).is_none());

    fixture.state.reader_lifecycle.mark_drained();
    finalizer.join().unwrap();
    let exit = lock_recover(&fixture.state.exit).clone().unwrap();
    assert_eq!(exit.reason, ExitReason::Lost);
    assert!(!fixture.state.output_blocked());
}

#[test]
fn reader_drained_at_the_blocked_threshold_remains_natural() {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.mark_drained();

    finalize_child_exit_scheduled(
        &fixture.state,
        None,
        None,
        FinalizationSchedule {
            drain_interval: Duration::ZERO,
            blocked_after: Duration::ZERO,
        },
    );

    let exit = lock_recover(&fixture.state.exit).clone().unwrap();
    assert_eq!(exit.reason, ExitReason::Natural);
    assert!(!fixture.state.reader_lifecycle.is_cancelled());
}

#[test]
fn explicit_lost_child_evidence_still_cancels_and_finishes() {
    let fixture = Fixture::new();
    fixture.state.begin_termination_for_test(ExitReason::Lost);
    assert!(fixture.state.reader_lifecycle.begin_attempt());
    let reader_state = Arc::clone(&fixture.state);
    let reader = std::thread::spawn(move || {
        while !reader_state.reader_lifecycle.is_cancelled() {
            std::thread::yield_now();
        }
        reader_state.reader_lifecycle.finish_attempt();
    });

    finalize_lost_unknown_exit(&fixture.state, Duration::ZERO);

    reader.join().unwrap();
    let exit = lock_recover(&fixture.state.exit).clone().unwrap();
    assert_eq!(exit.reason, ExitReason::Lost);
    assert!(fixture.state.reader_lifecycle.is_cancelled());
}
