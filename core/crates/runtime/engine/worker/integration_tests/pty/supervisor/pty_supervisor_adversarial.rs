use super::*;

const SUPERVISOR_MATRIX_NAMESPACE_LOSS_ATTEMPTS: usize = 2;

#[test]
fn pending_output_is_delivered_before_cancellation_stops_the_reader() {
    let result = run_supervisor_matrix(&[SupervisorMatrixStep::FailureWithPendingCancellation]);

    assert_eq!(result.attempts, SUPERVISOR_MATRIX_ATTEMPT_INCREMENT);
    assert!(result.restarts.is_empty());
    assert!(result.read_stopped);
    assert!(!result.pending_output);
    assert_eq!(result.output_sequence, OUTPUT_TEST_SEQUENCE);
    assert_eq!(result.exit_reason, None);
}

#[test]
fn retained_pending_output_settles_before_namespace_loss_termination() {
    let result = run_supervisor_matrix(&[
        SupervisorMatrixStep::FailureWithPendingNamespaceLoss,
        SupervisorMatrixStep::ChildExited,
    ]);

    assert_eq!(result.attempts, SUPERVISOR_MATRIX_NAMESPACE_LOSS_ATTEMPTS);
    assert!(result.restarts.is_empty());
    assert!(!result.read_stopped);
    assert!(result.output_blocked);
    assert!(result.namespace_loss_drain);
    assert!(!result.pending_output);
    assert_eq!(result.output_sequence, OUTPUT_TEST_SEQUENCE);
    assert_eq!(result.exit_reason, Some(ExitReason::Lost));
}

#[test]
fn cancelled_reader_state_rejects_a_late_attempt() {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.cancel();

    assert!(!fixture.state.reader_lifecycle.begin_attempt());
    let progress = lock_recover(&fixture.state.reader_lifecycle.progress);
    assert!(progress.read_stopped);
    assert!(!progress.eof.is_observed());
    assert!(progress.is_drained());
}

#[test]
fn supervisor_matrix_covers_exhaustion_and_final_drain_boundaries() {
    let exhausted_eof = run_supervisor_matrix(&[
        SupervisorMatrixStep::Eof {
            output: false,
            child_waited: false,
        },
        SupervisorMatrixStep::Eof {
            output: false,
            child_waited: false,
        },
        SupervisorMatrixStep::Eof {
            output: false,
            child_waited: true,
        },
        SupervisorMatrixStep::ChildExited,
    ]);
    assert_eq!(
        exhausted_eof.restarts.len(),
        usize::try_from(SUPERVISOR_MATRIX_EOF_LIMIT).unwrap()
    );

    let final_drain_eof = run_supervisor_matrix(&[
        SupervisorMatrixStep::Failure { child_waited: true },
        SupervisorMatrixStep::Eof {
            output: false,
            child_waited: true,
        },
        SupervisorMatrixStep::ChildExited,
    ]);
    assert_eq!(
        final_drain_eof.restarts,
        [(SUPERVISOR_MATRIX_FIRST_RESTART, AwaitingChildExit)]
    );

    let awaiting_exhaustion = run_supervisor_matrix(&[
        SupervisorMatrixStep::Failure {
            child_waited: false,
        },
        SupervisorMatrixStep::Failure {
            child_waited: false,
        },
        SupervisorMatrixStep::Failure {
            child_waited: false,
        },
        SupervisorMatrixStep::ChildExited,
    ]);
    assert!(!awaiting_exhaustion.output_blocked);

    let lost = run_supervisor_matrix(&[
        SupervisorMatrixStep::FailureWithNamespaceLoss,
        SupervisorMatrixStep::ChildExited,
    ]);
    assert_eq!(lost.exit_reason, Some(ExitReason::Lost));
    assert!(lost.output_blocked);
    assert!(lost.namespace_loss_drain);
}
