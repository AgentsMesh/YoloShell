use super::*;

#[test]
fn adversarial_state_writer_keeps_codex_authority_and_binding_aligned() {
    for state in [
        AgentState::Active,
        AgentState::Running,
        AgentState::WaitingForInput,
        AgentState::WaitingForPermission,
        AgentState::Exited,
        AgentState::Unknown,
    ] {
        let fixture = Fixture::new();
        let active = fixture.verified_authority(Generation::INITIAL.0);
        fixture.publish(&active).unwrap();

        crate::codex::authority_test_hook::persist_durable_state(&fixture.binding_path, state)
            .unwrap();

        let authority = fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap()
            .unwrap();
        let binding: AgentBinding = crate::paths::read_json(&fixture.binding_path).unwrap();
        assert_eq!(binding, authority.binding);
        assert_eq!(binding.state, state);
        match state {
            AgentState::Active
            | AgentState::Running
            | AgentState::WaitingForInput
            | AgentState::WaitingForPermission => {
                assert_eq!(authority.status, CodexAuthorityStatus::Active);
                assert_eq!(
                    binding.recovery_availability,
                    AgentRecoveryAvailability::Available
                );
                assert_eq!(binding.process_pid, Some(PROVIDER_PID));
                assert_eq!(binding.process_start_time, Some(PROVIDER_START));
            }
            AgentState::Exited => {
                assert_eq!(authority.status, CodexAuthorityStatus::Active);
                assert_eq!(
                    binding.recovery_availability,
                    AgentRecoveryAvailability::Available
                );
                assert!(binding.process_pid.is_none());
                assert!(binding.process_start_time.is_none());
            }
            AgentState::Unknown => {
                assert_eq!(authority.status, CodexAuthorityStatus::Unavailable);
                assert_eq!(
                    binding.recovery_availability,
                    AgentRecoveryAvailability::Unavailable
                );
            }
        }
    }
}

#[test]
fn adversarial_state_writer_reactivates_an_exited_authority() {
    let fixture = Fixture::new();
    let active = fixture.verified_authority(Generation::INITIAL.0);
    fixture.publish(&active).unwrap();

    for state in [AgentState::Exited, AgentState::Running] {
        crate::codex::authority_test_hook::persist_durable_state(&fixture.binding_path, state)
            .unwrap();
    }

    let authority = fixture
        .store()
        .load_in_namespace(&fixture.namespace)
        .unwrap()
        .unwrap();
    assert_eq!(authority.status, CodexAuthorityStatus::Active);
    assert_eq!(authority.binding.state, AgentState::Running);
    assert_eq!(authority.binding.process_pid, Some(PROVIDER_PID));
    assert_eq!(authority.binding.process_start_time, Some(PROVIDER_START));
}
