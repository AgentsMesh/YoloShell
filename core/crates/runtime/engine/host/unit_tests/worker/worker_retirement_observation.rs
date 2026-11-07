use super::*;

const RELEASED_RUNTIME_MATCH: &str = "matches_runtime=true";
const RELEASED_RUNTIME_MISMATCH: &str = "matches_runtime=false";
const RELEASED_DIAGNOSTIC_STATE: &str = "state=Some(Released)";
const RETIREMENT_IDENTITY_INCREMENT: u64 = 1;

#[path = "worker_retirement_control_phase.rs"]
mod control_phase;

#[tokio::test]
async fn retirement_diagnostic_distinguishes_acknowledgement_and_exit_observations() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    for (error, expected) in [
        (
            WorkerRetirementError::control(
                WorkerRetirementControlPhase::Authentication,
                WorkerRpcError::transport(record.auth_token.clone()),
            ),
            WorkerRetirementObservation::Control {
                phase: WorkerRetirementControlPhase::Authentication,
                failure: WorkerRetirementControlFailure::Transport,
            },
        ),
        (
            WorkerRetirementError::ExitTimeout,
            WorkerRetirementObservation::AcknowledgedExitTimeout,
        ),
        (
            WorkerRetirementError::SocketServing,
            WorkerRetirementObservation::AcknowledgedSocketServing,
        ),
    ] {
        assert_eq!(error.observation(), expected);
    }
    for observation in [
        WorkerRetirementObservation::NotRequested,
        WorkerRetirementObservation::Completed,
        WorkerRetirementObservation::Control {
            phase: WorkerRetirementControlPhase::Authentication,
            failure: WorkerRetirementControlFailure::Transport,
        },
        WorkerRetirementObservation::AcknowledgedExitTimeout,
        WorkerRetirementObservation::AcknowledgedSocketServing,
    ] {
        let error = host.unresolved_previous_runtime_after_retirement(&record, observation);
        assert!(error.message.contains(RELEASED_RUNTIME_MATCH));
        assert!(error.message.contains(&format!("{observation:?}")));
        assert!(!error.message.contains(&record.auth_token));
    }
}

#[tokio::test]
async fn matching_released_registry_key_does_not_hide_a_runtime_identity_mismatch() {
    let (_directory, host, mut record) = released_exit_fixture(ExitReason::Natural);
    record.child_start_time = record
        .child_start_time
        .checked_add(RETIREMENT_IDENTITY_INCREMENT)
        .unwrap();

    let error = host.unresolved_previous_runtime(&record);

    assert!(error.message.contains(RELEASED_DIAGNOSTIC_STATE));
    assert!(error.message.contains(RELEASED_RUNTIME_MISMATCH));
    assert!(!error.message.contains(&record.auth_token));
    assert!(
        host.validate_released_terminal_worker_shutdown(&record)
            .is_err()
    );
}
