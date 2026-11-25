#[test]
fn one_supervisor_instance_covers_recovery_decision_matrix() {
    let direct = run_supervisor_matrix(&[SupervisorMatrixStep::ChildExited]);
    assert_eq!(direct.attempts, SUPERVISOR_MATRIX_ATTEMPT_INCREMENT);
    assert!(direct.restarts.is_empty());

    let cancelled = run_supervisor_matrix(&[SupervisorMatrixStep::Cancelled]);
    assert!(!cancelled.eof_observed);
    assert!(!cancelled.kernel_eof);
    assert!(cancelled.read_stopped);

    let progress = run_supervisor_matrix(&[
        SupervisorMatrixStep::Eof {
            output: false,
            child_waited: false,
        },
        SupervisorMatrixStep::Eof {
            output: true,
            child_waited: false,
        },
        SupervisorMatrixStep::ChildExited,
    ]);
    assert_eq!(
        progress.restarts,
        [
            (SUPERVISOR_MATRIX_FIRST_RESTART, AwaitingChildExit),
            (SUPERVISOR_MATRIX_FIRST_RESTART, AwaitingChildExit)
        ]
    );

    let waited = run_supervisor_matrix(&[
        SupervisorMatrixStep::Eof {
            output: false,
            child_waited: true,
        },
        SupervisorMatrixStep::ChildExited,
    ]);
    assert_eq!(
        waited.restarts,
        [(SUPERVISOR_MATRIX_FIRST_RESTART, AwaitingChildExit)]
    );

    let failures = run_supervisor_matrix(&[
        SupervisorMatrixStep::Failure {
            child_waited: false,
        },
        SupervisorMatrixStep::Failure { child_waited: true },
        SupervisorMatrixStep::Failure { child_waited: true },
        SupervisorMatrixStep::Failure { child_waited: true },
        SupervisorMatrixStep::Failure { child_waited: true },
        SupervisorMatrixStep::ChildExited,
    ]);
    assert_eq!(
        failures.restarts,
        [
            (SUPERVISOR_MATRIX_FIRST_RESTART, AwaitingChildExit),
            (SUPERVISOR_MATRIX_SECOND_RESTART, AwaitingChildExit),
            (SUPERVISOR_MATRIX_FIRST_RESTART, FinalDrain),
            (SUPERVISOR_MATRIX_SECOND_RESTART, FinalDrain),
            (SUPERVISOR_MATRIX_THIRD_RESTART, FinalDrain)
        ]
    );
    assert!(!failures.eof_observed);
    assert!(!failures.read_stopped);
    assert!(!failures.output_blocked);
    assert_eq!(failures.exit_reason, None);

    let cancelled_after_exhaustion = run_supervisor_matrix(&[
        SupervisorMatrixStep::Failure {
            child_waited: false,
        },
        SupervisorMatrixStep::Failure { child_waited: true },
        SupervisorMatrixStep::Failure { child_waited: true },
        SupervisorMatrixStep::Failure { child_waited: true },
        SupervisorMatrixStep::Failure { child_waited: true },
        SupervisorMatrixStep::Cancelled,
    ]);
    assert!(cancelled_after_exhaustion.output_blocked);
    assert!(cancelled_after_exhaustion.read_stopped);

    let pending = run_supervisor_matrix(&[
        SupervisorMatrixStep::FailureWithPending,
        SupervisorMatrixStep::ChildExited,
    ]);
    assert!(pending.restarts.is_empty());
}

#[path = "pty_supervisor_adversarial.rs"]
mod adversarial;
