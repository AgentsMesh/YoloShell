use super::*;

const FUTURE_TIMESTAMP_INCREMENT: i64 = 1;

#[test]
fn active_authority_repairs_its_canonical_unavailable_predecessor() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();
    let predecessor = unavailable_predecessor(&active);
    fixture
        .namespace
        .write_json(&fixture.binding_path, &predecessor)
        .unwrap();

    repair(&fixture).unwrap();

    assert_binding(&fixture, &active.binding);
}

#[test]
fn unavailable_predecessor_with_a_future_timestamp_is_not_repaired() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();
    let mut predecessor = unavailable_predecessor(&active);
    predecessor.updated_at_ms += FUTURE_TIMESTAMP_INCREMENT;
    fixture
        .namespace
        .write_json(&fixture.binding_path, &predecessor)
        .unwrap();

    repair(&fixture).unwrap();

    assert_binding(&fixture, &predecessor);
}

fn unavailable_predecessor(authority: &CodexBindingAuthority) -> AgentBinding {
    let mut binding = authority.binding.clone();
    binding.state = AgentState::Unknown;
    binding.recovery_availability = AgentRecoveryAvailability::Unavailable;
    binding
}

fn repair(fixture: &Fixture) -> anyhow::Result<()> {
    fixture
        .store()
        .repair_active_binding_in_namespace_with_commit_fence(
            &fixture.namespace,
            &fixture.binding_path,
            fixture.terminal_id,
            fixture.runtime,
            |_| Ok(()),
            || Ok(()),
        )
}

fn assert_binding(fixture: &Fixture, expected: &AgentBinding) {
    let actual = fixture
        .namespace
        .read_json::<AgentBinding>(&fixture.binding_path)
        .unwrap();
    assert_eq!(&actual, expected);
}
