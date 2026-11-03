use super::*;

const FORGED_PROVIDER_PID: u32 = PROVIDER_PID + 1;
const FORGED_PROVIDER_START: u64 = PROVIDER_START + 1;

#[test]
fn current_runtime_can_canonically_reactivate_a_finalized_version() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();
    let terminal = active.unavailable().unwrap();
    fixture.publish(&terminal).unwrap();
    let mut reactivated = active;
    reactivated.binding.updated_at_ms = terminal.binding.updated_at_ms;

    fixture.publish(&reactivated).unwrap();
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(reactivated)
    );
}

#[test]
fn exited_reactivation_cannot_replace_provider_identity() {
    let fixture = Fixture::new();
    let unavailable = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Unavailable);
    fixture.publish(&unavailable).unwrap();
    let mut binding = unavailable.binding.clone();
    binding.state = AgentState::Exited;
    binding.recovery_availability = AgentRecoveryAvailability::Available;
    binding.process_pid = None;
    binding.process_start_time = None;
    let forged = CodexBindingAuthority::new_with_verified_resume_identity(
        fixture.runtime,
        ProcessIdentity::new(FORGED_PROVIDER_PID, FORGED_PROVIDER_START),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        unavailable.version,
        CodexAuthorityStatus::Active,
        binding,
        TEST_RESUME_BOOT,
    )
    .unwrap();

    assert!(fixture.publish(&forged).is_err());
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(unavailable)
    );
}
