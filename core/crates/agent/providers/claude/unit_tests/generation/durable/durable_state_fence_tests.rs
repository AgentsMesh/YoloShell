use super::support::*;
use crate::agent::claude::generation::adversarial_test_support::{
    persist_durable_state, prepare_durable_state_for_test,
};
use crate::agent::{AgentBindingStore, AgentRecoveryAvailability, AgentState};

#[test]
fn a_revoked_publication_can_be_retargeted_and_converges() {
    let fixture = published_fixture();
    let committed = fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    fixture
        .store
        .fence_binding_if_active_in_namespace(&fixture.lease, fixture.authentication(), &committed)
        .unwrap();
    let generation_before = publication_generation(&fixture);

    persist_durable_state(&fixture.binding_path(), AgentState::Exited).unwrap();
    let generation_after = publication_generation(&fixture);
    assert_eq!(
        generation_after,
        generation_before
            .checked_add(super::super::constants::REVISION_INCREMENT)
            .unwrap()
    );
    let exited = fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    assert_eq!(exited.state, AgentState::Exited);

    persist_durable_state(&fixture.binding_path(), AgentState::Exited).unwrap();
    assert_eq!(publication_generation(&fixture), generation_after);
}

#[test]
fn a_pending_transition_can_be_fenced_without_installing_its_target() {
    let fixture = published_fixture();
    let binding_store = AgentBindingStore::new(fixture.binding_path());
    let binding_before = binding_store
        .read_raw_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    let pending =
        prepare_durable_state_for_test(&fixture.binding_path(), AgentState::WaitingForInput)
            .unwrap();
    let generation_before = publication_generation(&fixture);

    fixture
        .store
        .fence_binding_if_active_in_namespace(&fixture.lease, fixture.authentication(), &pending)
        .unwrap();

    let set = fixture.store.load_set(&fixture.lease).unwrap().unwrap();
    assert!(set.pending_binding_transition.is_none());
    assert!(set.binding_publication.is_none());
    assert_eq!(set.active().unwrap(), &pending);
    assert_eq!(
        set.publication_generation,
        generation_before
            .checked_add(super::super::constants::REVISION_INCREMENT)
            .unwrap()
    );
    assert_eq!(
        binding_store
            .read_raw_in_namespace(&fixture.lease)
            .unwrap()
            .unwrap(),
        binding_before
    );
}

#[test]
fn a_committed_state_only_binding_divergence_is_republished() {
    let fixture = published_fixture();
    let generation_before = publication_generation(&fixture);
    let binding_store = AgentBindingStore::new(fixture.binding_path());
    let mut stale = binding_store
        .read_raw_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    stale.state = AgentState::Unknown;
    stale.recovery_availability = AgentRecoveryAvailability::Unavailable;
    fixture
        .lease
        .write_json(&fixture.binding_path(), &stale)
        .unwrap();

    persist_durable_state(&fixture.binding_path(), AgentState::Running).unwrap();

    let authority = fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    let binding = binding_store
        .read_raw_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    assert!(authority.matches_binding(&binding));
    assert_eq!(binding.state, AgentState::Running);
    assert_eq!(
        publication_generation(&fixture),
        generation_before
            .checked_add(super::super::constants::REVISION_INCREMENT)
            .unwrap()
    );
}

fn published_fixture() -> Fixture {
    let fixture = Fixture::new();
    let session_id = fixture.begin.provider_session_id.clone();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let authority = fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &fixture.start(&session_id, super::super::SessionStartSource::Fresh),
        )
        .unwrap()
        .0;
    crate::agent::AgentBindingStore::new(fixture.binding_path())
        .save_authoritative_in_namespace(
            &fixture.lease,
            &binding(&authority),
            &authority,
            fixture.runtime(),
        )
        .unwrap();
    fixture
}

fn publication_generation(fixture: &Fixture) -> u64 {
    fixture
        .store
        .publication_generation_in_namespace(&fixture.lease)
        .unwrap()
}
