pub(crate) fn supervise_reader(initial_reader: Box<dyn Read + Send>, state: Arc<WorkerState>) {
    let restart_state = Arc::clone(&state);
    supervise_reader_with(
        initial_reader,
        state,
        PTY_READER_UNCONFIRMED_EOF_REOPEN_LIMIT,
        PTY_READER_FAILURE_RESTART_LIMIT,
        run_reader_attempt,
        move |failure, failure_count, phase| {
            restart_reader_after(
                failure,
                failure_count,
                phase,
                ReaderRestartKind::Read,
                &restart_state,
            )
        },
    );
}

pub(crate) fn stop_cancelled_reader(state: &WorkerState) -> bool {
    if !state.reader_lifecycle.is_cancelled() {
        return false;
    }
    state.reader_lifecycle.mark_read_stopped();
    true
}

pub(crate) fn supervise_reader_with<Attempt, Restart>(
    initial_reader: Box<dyn Read + Send>,
    state: Arc<WorkerState>,
    unconfirmed_eof_reopen_limit: u32,
    reader_failure_restart_limit: u32,
    attempt: Attempt,
    mut restart: Restart,
) where
    Attempt: FnMut(Box<dyn Read + Send>, Arc<WorkerState>) -> Result<ReaderLoopOutcome>,
    Restart: FnMut(&supervision::TaskFailure, u32, ReaderRestartPhase),
{
    let mut reader_custody =
        ReaderSupervisorCustody::new(initial_reader, Arc::clone(&state.reader_lifecycle));
    // A stateful attempt may retain its reader; retire it before publishing custody completion.
    let mut attempt = attempt;
    let mut recovery = ReaderRecoveryBudget::default();
    loop {
        if state.reader_lifecycle.has_pending() {
            let restart_phase = recovery.begin_attempt(&state.child_waited);
            let pending = supervision::catch_thread(supervision::PTY_READER_TASK, || {
                let _attempt = ReaderAttemptGuard::start(&state.reader_lifecycle);
                deliver_pending_with_retry(&state)
            });
            match pending {
                Ok(PendingDelivery::Delivered) => {
                    recovery.reset();
                }
                Ok(PendingDelivery::NamespaceLost) => {
                    let _ = state.begin_output_drain_after_namespace_loss();
                    recovery.reset();
                }
                Err(failure) => {
                    let failure_count = recovery.record_backoff_failure();
                    restart_reader_after(
                        &failure,
                        failure_count,
                        restart_phase,
                        ReaderRestartKind::PendingDelivery,
                        &state,
                    );
                    continue;
                }
            }
        }
        if stop_cancelled_reader(&state) {
            return;
        }
        let restart_phase = recovery.begin_attempt(&state.child_waited);
        let result = supervision::catch_thread(supervision::PTY_READER_TASK, || {
            let reader = next_reader(&mut reader_custody, &state)?;
            attempt(reader, Arc::clone(&state))
        });
        if matches!(&result, Ok(ReaderLoopOutcome::ChildExited)) {
            // Only a reader that observed the child exit proves the restart
            // condition resolved; cancellation and a bare EOF do not.
            state.clear_output_blocked(OutputBlockCause::ReaderRestartsExhausted);
        }
        let failure = match result {
            Ok(ReaderLoopOutcome::ChildExited) => return,
            Ok(ReaderLoopOutcome::Cancelled) => {
                state.reader_lifecycle.mark_read_stopped();
                return;
            }
            Ok(ReaderLoopOutcome::UnexpectedEof { output_observed }) => {
                recovery.reset_reader_failures();
                if output_observed {
                    recovery.reset();
                }
                if restart_phase == ReaderRestartPhase::FinalDrain {
                    continue;
                }
                if recovery.eof_reopens_exhausted(unconfirmed_eof_reopen_limit) {
                    tracing::warn!(
                        terminal_id = %state.terminal_id(),
                        PTY_READER_RECOVERY_EXHAUSTED_DETAIL
                    );
                    state
                        .reader_lifecycle
                        .wait_for_child_exit_confirmation(&state.child_waited);
                    if stop_cancelled_reader(&state) {
                        return;
                    }
                    recovery.reset();
                    continue;
                }
                recovery.record_eof_reopen();
                supervision::TaskFailure::message(
                    supervision::PTY_READER_TASK,
                    PTY_READER_UNEXPECTED_EOF_DETAIL,
                )
            }
            Err(failure) => {
                recovery.reset_eof_reopens();
                if state.reader_lifecycle.has_pending() {
                    supervision::report_contained(&failure);
                    continue;
                }
                let draining_after_namespace_loss = state.begin_output_drain_after_namespace_loss();
                if recovery.reader_restarts_exhausted(reader_failure_restart_limit) {
                    if restart_phase == ReaderRestartPhase::FinalDrain {
                        state.mark_output_blocked(OutputBlockCause::ReaderRestartsExhausted);
                        if draining_after_namespace_loss {
                            supervision::report_contained(&failure);
                            state.reader_lifecycle.mark_read_stopped();
                            return;
                        }
                    }
                    tracing::warn!(
                        terminal_id = %state.terminal_id(),
                        PTY_READER_FAILURE_RETRY_DETAIL
                    );
                    recovery.reset_reader_failures();
                }
                recovery.record_reader_restart();
                failure
            }
        };
        let failure_count = recovery.record_backoff_failure();
        restart(&failure, failure_count, restart_phase);
    }
}

pub(crate) fn restart_reader_after(
    failure: &supervision::TaskFailure,
    failure_count: u32,
    phase: ReaderRestartPhase,
    kind: ReaderRestartKind,
    state: &WorkerState,
) {
    let delay = supervision::report_restart(
        failure,
        supervision::RestartPolicy::default(),
        failure_count,
    );
    state
        .reader_lifecycle
        .wait_for_restart(delay, &state.child_waited, phase, kind);
}

pub(crate) fn next_reader(
    custody: &mut ReaderSupervisorCustody,
    state: &WorkerState,
) -> Result<Box<dyn Read + Send>> {
    if let Some(reader) = custody.take_initial() {
        return Ok(reader);
    }
    lock_recover(&state.master)
        .as_ref()
        .context("PTY master unavailable while restarting reader")?
        .try_clone_reader()
        .context("clone PTY reader for restart")
}
