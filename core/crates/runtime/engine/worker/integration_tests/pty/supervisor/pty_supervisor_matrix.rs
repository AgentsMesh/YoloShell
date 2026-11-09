use crate::worker::tests::*;
const SUPERVISOR_MATRIX_EOF_LIMIT: u32 = 2;
const SUPERVISOR_MATRIX_FAILURE_LIMIT: u32 = 2;
const SUPERVISOR_MATRIX_FIRST_RESTART: u32 = 1;
const SUPERVISOR_MATRIX_SECOND_RESTART: u32 = 2;
const SUPERVISOR_MATRIX_THIRD_RESTART: u32 = 3;
const SUPERVISOR_MATRIX_ATTEMPT_INCREMENT: usize = 1;
const SUPERVISOR_MATRIX_PENDING_OUTPUT: &[u8] = b"matrix-pending";
const SUPERVISOR_MATRIX_FAILURE: &str = "scripted reader failure";
const SUPERVISOR_MATRIX_EXHAUSTED: &str = "supervisor matrix script exhausted";
const SUPERVISOR_MATRIX_MOVED_SESSION: &str = "supervisor-matrix-moved-session";
use ReaderRestartPhase::{AwaitingChildExit, FinalDrain};
include!("../../../test_support/integration/pty/pty_supervisor_matrix_types.rs");
struct SupervisorMatrixResult {
    attempts: usize,
    eof_observed: bool,
    kernel_eof: bool,
    read_stopped: bool,
    output_blocked: bool,
    namespace_loss_drain: bool,
    pending_output: bool,
    output_sequence: u64,
    exit_reason: Option<ExitReason>,
    restarts: Vec<(u32, ReaderRestartPhase)>,
}
fn run_supervisor_matrix(steps: &[SupervisorMatrixStep]) -> SupervisorMatrixResult {
    let fixture = Fixture::new();
    let script = Arc::new(Mutex::new(std::collections::VecDeque::from(steps.to_vec())));
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::default());
    let restarts = Arc::new(Mutex::new(Vec::new()));
    let attempt_script = Arc::clone(&script);
    let attempt_count = Arc::clone(&attempts);
    let restart_counts = Arc::clone(&restarts);
    supervise_reader_with(
        Box::new(io::empty()),
        Arc::clone(&fixture.state),
        SUPERVISOR_MATRIX_EOF_LIMIT,
        SUPERVISOR_MATRIX_FAILURE_LIMIT,
        move |_reader, state| {
            attempt_count.fetch_add(SUPERVISOR_MATRIX_ATTEMPT_INCREMENT, Ordering::AcqRel);
            let step = attempt_script
                .lock()
                .unwrap()
                .pop_front()
                .expect(SUPERVISOR_MATRIX_EXHAUSTED);
            match step {
                SupervisorMatrixStep::ChildExited => Ok(ReaderLoopOutcome::ChildExited),
                SupervisorMatrixStep::Cancelled => {
                    state.reader_lifecycle.mark_eof_observed();
                    Ok(ReaderLoopOutcome::Cancelled)
                }
                SupervisorMatrixStep::Eof {
                    output,
                    child_waited,
                } => {
                    set_matrix_child_waited(&state, child_waited);
                    Ok(ReaderLoopOutcome::UnexpectedEof {
                        output_observed: output,
                    })
                }
                SupervisorMatrixStep::Failure { child_waited } => {
                    set_matrix_child_waited(&state, child_waited);
                    Err(anyhow::anyhow!(SUPERVISOR_MATRIX_FAILURE))
                }
                SupervisorMatrixStep::FailureWithPending => {
                    stage_reader_output(&state, SUPERVISOR_MATRIX_PENDING_OUTPUT)?;
                    Err(anyhow::anyhow!(SUPERVISOR_MATRIX_FAILURE))
                }
                SupervisorMatrixStep::FailureWithPendingCancellation => {
                    stage_reader_output(&state, SUPERVISOR_MATRIX_PENDING_OUTPUT)?;
                    state.reader_lifecycle.cancel();
                    Err(anyhow::anyhow!(SUPERVISOR_MATRIX_FAILURE))
                }
                SupervisorMatrixStep::FailureWithPendingNamespaceLoss => {
                    stage_reader_output(&state, SUPERVISOR_MATRIX_PENDING_OUTPUT)?;
                    let session = state.paths.session_dir(state.terminal_id());
                    std::fs::rename(
                        &session,
                        session.with_file_name(SUPERVISOR_MATRIX_MOVED_SESSION),
                    )?;
                    Err(anyhow::anyhow!(SUPERVISOR_MATRIX_FAILURE))
                }
                SupervisorMatrixStep::FailureWithNamespaceLoss => {
                    let session = state.paths.session_dir(state.terminal_id());
                    std::fs::rename(
                        &session,
                        session.with_file_name(SUPERVISOR_MATRIX_MOVED_SESSION),
                    )?;
                    Err(anyhow::anyhow!(SUPERVISOR_MATRIX_FAILURE))
                }
            }
        },
        move |_failure, count, phase| restart_counts.lock().unwrap().push((count, phase)),
    );
    assert!(script.lock().unwrap().is_empty());
    let (eof_observed, kernel_eof, read_stopped) = {
        let progress = lock_recover(&fixture.state.reader_lifecycle.progress);
        (
            progress.eof.is_observed(),
            progress.eof.has_authority(),
            progress.read_stopped,
        )
    };
    let pending_output = fixture.state.reader_lifecycle.has_pending();
    let output_sequence = fixture.state.journal.lock().unwrap().last_sequence();
    SupervisorMatrixResult {
        attempts: attempts.load(Ordering::Acquire),
        eof_observed,
        kernel_eof,
        read_stopped,
        output_blocked: fixture.state.output_blocked(),
        namespace_loss_drain: fixture.state.reader_lifecycle.drains_after_namespace_loss(),
        pending_output,
        output_sequence,
        exit_reason: fixture.state.termination_reason(),
        restarts: restarts.lock().unwrap().clone(),
    }
}
fn set_matrix_child_waited(state: &WorkerState, waited: bool) {
    if waited {
        state.child_waited.store(true, Ordering::Release);
        state.reader_lifecycle.notify_child_exit_confirmation();
    }
}

include!("pty_supervisor_decision_matrix.rs");
