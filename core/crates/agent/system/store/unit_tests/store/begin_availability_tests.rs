use super::*;
use crate::agent::AgentRecoveryAvailability;
use crate::agent::claude::generation::GenerationApplyDisposition;

#[path = "../../test_support/store/begin_availability.rs"]
mod support;
use support::Fixture;

struct Case {
    state: AgentState,
    availability: AgentRecoveryAvailability,
    invalidated_state: AgentState,
}

const CASES: &[Case] = &[
    Case {
        state: AgentState::Running,
        availability: AgentRecoveryAvailability::Available,
        invalidated_state: AgentState::Unknown,
    },
    Case {
        state: AgentState::WaitingForInput,
        availability: AgentRecoveryAvailability::Available,
        invalidated_state: AgentState::Unknown,
    },
    Case {
        state: AgentState::Exited,
        availability: AgentRecoveryAvailability::Available,
        invalidated_state: AgentState::Exited,
    },
    Case {
        state: AgentState::Unknown,
        availability: AgentRecoveryAvailability::Available,
        invalidated_state: AgentState::Unknown,
    },
    Case {
        state: AgentState::Running,
        availability: AgentRecoveryAvailability::UnmanagedLaunch,
        invalidated_state: AgentState::Unknown,
    },
];

#[test]
fn managed_begin_durably_invalidates_every_stale_recovery_state() {
    for case in CASES {
        let fixture = Fixture::new(case.state, case.availability);
        let invalidated = fixture.invalidate();
        assert_eq!(invalidated.state, case.invalidated_state);
        assert_eq!(
            invalidated.recovery_availability,
            AgentRecoveryAvailability::Unavailable
        );
        assert_eq!(fixture.persisted(), invalidated);

        fixture.begin_generation();
        assert_eq!(fixture.persisted(), invalidated);
    }
}

#[test]
fn only_authenticated_session_start_promotes_recovery_availability() {
    let fixture = Fixture::new(
        AgentState::Exited,
        AgentRecoveryAvailability::UnmanagedLaunch,
    );
    let invalidated = fixture.invalidate();
    let authority = fixture.begin_generation();
    assert_eq!(fixture.persisted(), invalidated);

    let event = fixture.session_start();
    let (authority, disposition) = fixture
        .generations()
        .apply_in_namespace(&fixture.lease, authority.authentication(), &event)
        .unwrap();
    assert_eq!(disposition, GenerationApplyDisposition::Applied);
    let promoted = authority.binding(Some(&invalidated)).unwrap();
    fixture
        .bindings
        .save_authoritative_in_namespace(&fixture.lease, &promoted, &authority, fixture.runtime())
        .unwrap();
    assert_eq!(fixture.persisted().state, AgentState::Running);
    assert_eq!(
        fixture.persisted().recovery_availability,
        AgentRecoveryAvailability::Available
    );
}

#[test]
fn unmanaged_predecessor_converges_through_the_durable_publisher() {
    let fixture = Fixture::new(
        AgentState::Running,
        AgentRecoveryAvailability::UnmanagedLaunch,
    );
    let invalidated = fixture.invalidate();
    assert_eq!(invalidated.state, AgentState::Unknown);
    assert_eq!(
        invalidated.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
    let initial = fixture.begin_generation();
    assert_eq!(fixture.persisted(), invalidated);

    let event = fixture.session_start();
    let generations = fixture.generations();
    let (applied, disposition, _) = generations
        .transition_foreground_durable_in_namespace(
            &fixture.lease,
            initial.authentication(),
            &initial,
            crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
            &event,
            Some(fixture.transcript_identity()),
        )
        .unwrap();
    assert_eq!(disposition, GenerationApplyDisposition::Applied);

    let (committed, binding) = generations
        .publish_binding_state_in_namespace(
            &fixture.lease,
            fixture.bindings.path(),
            applied.authentication(),
            &applied,
            false,
            |_| Ok(()),
        )
        .unwrap();
    assert_eq!(committed.binding_revision, Some(committed.revision));
    assert!(committed.matches_binding(&binding));
    assert_eq!(
        binding.recovery_availability,
        AgentRecoveryAvailability::Available
    );
    assert_eq!(fixture.persisted(), binding);
    assert_eq!(
        generations.load_in_namespace(&fixture.lease).unwrap(),
        Some(committed)
    );
}
