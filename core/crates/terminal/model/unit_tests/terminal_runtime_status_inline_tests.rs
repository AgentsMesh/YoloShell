use super::*;

#[test]
fn exact_status_and_completed_stop_observations_are_accepted() {
    let statuses = [
        TerminalRuntimeStatus::Starting,
        TerminalRuntimeStatus::Running,
        TerminalRuntimeStatus::Finalizing,
        TerminalRuntimeStatus::OutputBlocked,
        TerminalRuntimeStatus::Exited,
        TerminalRuntimeStatus::Unreachable,
    ];
    for expected in statuses {
        for observed in statuses {
            let completed_stop = observed == TerminalRuntimeStatus::Exited;
            assert_eq!(
                expected.accepts_exact_termination_observation(observed),
                expected == observed || completed_stop,
                "expected={expected:?}, observed={observed:?}"
            );
        }
    }
}
