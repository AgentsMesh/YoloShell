const READER_PHASE_FAILURE_DETAIL: &str = "injected reader restart phase failure";
const READER_PHASE_ATTEMPT_INCREMENT: usize = 1;
const READER_PHASE_FIRST_RESTART: u32 = 1;
const READER_PHASE_SECOND_RESTART: u32 = 2;
const READER_PHASE_EOF_ATTEMPT: usize = 2;
const READER_PHASE_DRAIN_COMPLETE_ATTEMPT: usize = 8;
const READER_PHASE_SATURATED_RESTART: u32 = 6;

#[test]
fn final_drain_starts_with_fresh_recovery_budgets() {
    let fixture = Fixture::new();
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::default());
    let restarts = Arc::new(Mutex::new(Vec::new()));
    let attempt_count = Arc::clone(&attempts);
    let restart_records = Arc::clone(&restarts);
    let restart_state = Arc::clone(&fixture.state);

    supervise_reader_with(
        Box::new(io::empty()),
        Arc::clone(&fixture.state),
        PTY_READER_UNCONFIRMED_EOF_REOPEN_LIMIT,
        PTY_READER_FAILURE_RESTART_LIMIT,
        move |_reader, state| {
            let attempt = attempt_count.fetch_add(READER_PHASE_ATTEMPT_INCREMENT, Ordering::AcqRel);
            if attempt == READER_PHASE_EOF_ATTEMPT {
                return Ok(ReaderLoopOutcome::UnexpectedEof {
                    output_observed: false,
                });
            }
            if attempt < READER_PHASE_DRAIN_COMPLETE_ATTEMPT {
                return Err(anyhow::anyhow!(READER_PHASE_FAILURE_DETAIL));
            }
            state.reader_lifecycle.mark_eof_observed();
            Ok(ReaderLoopOutcome::ChildExited)
        },
        move |_failure, count, phase| {
            restart_records.lock().unwrap().push((count, phase));
            if count == READER_PHASE_SATURATED_RESTART
                && phase == ReaderRestartPhase::AwaitingChildExit
            {
                restart_state.child_waited.store(true, Ordering::Release);
                restart_state
                    .reader_lifecycle
                    .notify_child_exit_confirmation();
            }
        },
    );

    let records = restarts.lock().unwrap();
    let final_drain_start = usize::try_from(READER_PHASE_SATURATED_RESTART).unwrap();
    let saturated_index = final_drain_start.saturating_sub(READER_PHASE_ATTEMPT_INCREMENT);
    assert_eq!(records.len(), READER_PHASE_DRAIN_COMPLETE_ATTEMPT);
    assert!(
        records[..final_drain_start]
            .iter()
            .all(|(_, phase)| *phase == ReaderRestartPhase::AwaitingChildExit)
    );
    assert_eq!(
        records[saturated_index],
        (
            READER_PHASE_SATURATED_RESTART,
            ReaderRestartPhase::AwaitingChildExit
        )
    );
    assert_eq!(
        supervision::RestartPolicy::default().delay(records[saturated_index].0),
        SUPERVISION_RESTART_MAX_DELAY
    );
    assert_eq!(
        &records[final_drain_start..],
        &[
            (READER_PHASE_FIRST_RESTART, ReaderRestartPhase::FinalDrain),
            (READER_PHASE_SECOND_RESTART, ReaderRestartPhase::FinalDrain),
        ]
    );
    drop(records);
    assert_eq!(fixture.state.termination_reason(), None);
    assert!(lock_recover(&fixture.state.reader_lifecycle.progress).is_drained());
}
