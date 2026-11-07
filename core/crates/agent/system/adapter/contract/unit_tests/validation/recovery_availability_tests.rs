use super::*;
use crate::SessionGenerationPolicy;
use yolo_agent_contract::{AgentRecoveryAvailability, AgentState};

struct Case {
    state: AgentState,
    availability: AgentRecoveryAvailability,
    valid: bool,
}

const HOOK_REQUIRED_CASES: &[Case] = &[
    Case::valid(AgentState::Active, AgentRecoveryAvailability::Available),
    Case::invalid(AgentState::Active, AgentRecoveryAvailability::Unavailable),
    Case::invalid(
        AgentState::Active,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::valid(
        AgentState::Unknown,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::valid(AgentState::Unknown, AgentRecoveryAvailability::Unavailable),
    Case::valid(AgentState::Unknown, AgentRecoveryAvailability::Available),
    Case::valid(AgentState::Running, AgentRecoveryAvailability::Available),
    Case::invalid(
        AgentState::Running,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::invalid(AgentState::Running, AgentRecoveryAvailability::Unavailable),
    Case::valid(
        AgentState::WaitingForInput,
        AgentRecoveryAvailability::Available,
    ),
    Case::invalid(
        AgentState::WaitingForInput,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::invalid(
        AgentState::WaitingForInput,
        AgentRecoveryAvailability::Unavailable,
    ),
    Case::valid(
        AgentState::WaitingForPermission,
        AgentRecoveryAvailability::Available,
    ),
    Case::invalid(
        AgentState::WaitingForPermission,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::invalid(
        AgentState::WaitingForPermission,
        AgentRecoveryAvailability::Unavailable,
    ),
    Case::valid(AgentState::Exited, AgentRecoveryAvailability::Available),
    Case::invalid(
        AgentState::Exited,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::valid(AgentState::Exited, AgentRecoveryAvailability::Unavailable),
];

const AUTHORITATIVE_ONLY_CASES: &[Case] = &[
    Case::valid(AgentState::Active, AgentRecoveryAvailability::Available),
    Case::invalid(AgentState::Active, AgentRecoveryAvailability::Unavailable),
    Case::invalid(
        AgentState::Active,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::invalid(AgentState::Unknown, AgentRecoveryAvailability::Available),
    Case::invalid(
        AgentState::Unknown,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::valid(AgentState::Unknown, AgentRecoveryAvailability::Unavailable),
    Case::valid(AgentState::Running, AgentRecoveryAvailability::Available),
    Case::invalid(
        AgentState::Running,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::invalid(AgentState::Running, AgentRecoveryAvailability::Unavailable),
    Case::valid(
        AgentState::WaitingForInput,
        AgentRecoveryAvailability::Available,
    ),
    Case::invalid(
        AgentState::WaitingForInput,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::invalid(
        AgentState::WaitingForInput,
        AgentRecoveryAvailability::Unavailable,
    ),
    Case::valid(
        AgentState::WaitingForPermission,
        AgentRecoveryAvailability::Available,
    ),
    Case::invalid(
        AgentState::WaitingForPermission,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::invalid(
        AgentState::WaitingForPermission,
        AgentRecoveryAvailability::Unavailable,
    ),
    Case::valid(AgentState::Exited, AgentRecoveryAvailability::Available),
    Case::invalid(
        AgentState::Exited,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ),
    Case::valid(AgentState::Exited, AgentRecoveryAvailability::Unavailable),
];

impl Case {
    const fn valid(state: AgentState, availability: AgentRecoveryAvailability) -> Self {
        Self {
            state,
            availability,
            valid: true,
        }
    }

    const fn invalid(state: AgentState, availability: AgentRecoveryAvailability) -> Self {
        Self {
            state,
            availability,
            valid: false,
        }
    }
}

#[test]
fn hook_required_recovery_matrix_is_closed() {
    let (_directory, _context, mut binding, _root) = fixture();
    let adapter = super::evidence_tests::hook_required_adapter();
    for case in HOOK_REQUIRED_CASES {
        binding.state = case.state;
        binding.recovery_availability = case.availability;
        let result = validate_recovery_availability(&adapter, &binding);
        assert_eq!(result.is_ok(), case.valid, "{:?}", case.state);
        if !case.valid {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains(crate::constants::message::RECOVERY_AVAILABILITY)
            );
        }
    }
}

#[test]
fn authoritative_only_recovery_matrix_is_closed() {
    let (_directory, _context, mut binding, _root) = fixture();
    for case in AUTHORITATIVE_ONLY_CASES {
        binding.state = case.state;
        binding.recovery_availability = case.availability;
        assert_eq!(
            SessionGenerationPolicy::AuthoritativeOnly.recovery_availability_is_valid(&binding),
            case.valid,
            "{:?}",
            case.state
        );
    }
}
