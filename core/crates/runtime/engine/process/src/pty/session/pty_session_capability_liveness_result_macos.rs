use super::CapabilityScanAdvance;

fn premature_release_decision(authorized: bool) -> MacosCapabilityLivenessDecision {
    if authorized {
        MacosCapabilityLivenessDecision::Scan {
            state: MacosCapabilityLivenessState::PrematurelyReleased,
            require_pending: false,
            reset_released: false,
        }
    } else {
        MacosCapabilityLivenessDecision::Pending(MacosCapabilityLivenessState::PrematurelyReleased)
    }
}

pub(super) fn complete_authorized_scan(
    state: MacosCapabilityLivenessState,
    status: CapabilityScanStatus,
) -> MacosCapabilityLivenessState {
    if state == MacosCapabilityLivenessState::ReleasedNeedsScan
        && status == CapabilityScanStatus::Complete
    {
        MacosCapabilityLivenessState::ReleasedAfterRootExit
    } else {
        state
    }
}

pub(super) fn terminal(
    status: CapabilityScanStatus,
    state: MacosCapabilityLivenessState,
) -> CapabilityScanResult {
    CapabilityScanResult {
        candidates: CapabilityCandidates {
            status,
            ..CapabilityCandidates::default()
        },
        advance: CapabilityScanAdvance::macos_terminal(state),
        scheduling_status: status,
        coverage_status: status,
    }
}
