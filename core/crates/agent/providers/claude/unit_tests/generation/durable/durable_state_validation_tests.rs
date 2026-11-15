use super::durable_state_publication_tests::published_fixture;
use crate::agent::claude::generation::adversarial_test_support::prepare_durable_state_for_test;
use crate::agent::claude::generation::constants::INITIAL_PUBLICATION_GENERATION;
use crate::agent::{AgentBinding, AgentState};

const REJECTED_BINDING: &str = "injected binding validation rejection";
const FORGED_PREDECESSOR_PUBLICATION_GENERATION: u64 =
    INITIAL_PUBLICATION_GENERATION - super::super::constants::REVISION_INCREMENT;

#[test]
fn rejecting_pending_target_preserves_every_durable_publication_record() {
    let fixture = published_fixture();
    let pending =
        prepare_durable_state_for_test(&fixture.binding_path(), AgentState::WaitingForInput)
            .unwrap();
    let before = durable_snapshot(&fixture);
    assert!(before.set.pending_binding_transition.is_some());
    assert!(before.set.binding_publication.is_none());

    let error = fixture
        .store
        .publish_binding_state_in_namespace(
            &fixture.lease,
            &fixture.binding_path(),
            fixture.authentication(),
            &pending,
            false,
            |_| anyhow::bail!(REJECTED_BINDING),
        )
        .unwrap_err();

    assert!(error.to_string().contains(REJECTED_BINDING));
    assert_eq!(durable_snapshot(&fixture), before);
}

#[test]
fn rejecting_forced_refresh_does_not_revoke_a_published_binding() {
    let fixture = published_fixture();
    let authority = fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    let before = durable_snapshot(&fixture);
    assert!(before.set.binding_publication.is_some());

    let error = fixture
        .store
        .publish_binding_state_in_namespace(
            &fixture.lease,
            &fixture.binding_path(),
            fixture.authentication(),
            &authority,
            true,
            |_| anyhow::bail!(REJECTED_BINDING),
        )
        .unwrap_err();

    assert!(error.to_string().contains(REJECTED_BINDING));
    assert_eq!(durable_snapshot(&fixture), before);
}

#[test]
fn predecessor_publication_generation_cannot_be_below_initial_even_with_consistent_relation() {
    let fixture = published_fixture();
    prepare_durable_state_for_test(&fixture.binding_path(), AgentState::WaitingForInput).unwrap();
    let mut forged: serde_json::Value = fixture.lease.read_json(fixture.store.path()).unwrap();
    let transition = forged
        .get_mut("pending_binding_transition")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap();
    transition.insert(
        "predecessor_publication_generation".to_owned(),
        FORGED_PREDECESSOR_PUBLICATION_GENERATION.into(),
    );
    transition.insert(
        "publication_generation".to_owned(),
        INITIAL_PUBLICATION_GENERATION.into(),
    );
    transition
        .get_mut("predecessor_publication")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap()
        .insert(
            "publication_generation".to_owned(),
            FORGED_PREDECESSOR_PUBLICATION_GENERATION.into(),
        );
    fixture
        .lease
        .write_json(fixture.store.path(), &forged)
        .unwrap();

    assert!(fixture.store.load_in_namespace(&fixture.lease).is_err());
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct DurableSnapshot {
    authority_bytes: Vec<u8>,
    binding_bytes: Vec<u8>,
    binding_identity: crate::paths::identity::FileIdentity,
    set: super::super::authority_set::ClaudeInvocationAuthorities,
}

pub(super) fn durable_snapshot(fixture: &super::support::Fixture) -> DurableSnapshot {
    let binding_path = fixture.binding_path();
    let (_, binding_identity) = fixture
        .lease
        .read_json_with_identity::<AgentBinding>(&binding_path)
        .unwrap();
    DurableSnapshot {
        authority_bytes: std::fs::read(fixture.store.path()).unwrap(),
        binding_bytes: std::fs::read(binding_path).unwrap(),
        binding_identity,
        set: fixture.store.load_set(&fixture.lease).unwrap().unwrap(),
    }
}
