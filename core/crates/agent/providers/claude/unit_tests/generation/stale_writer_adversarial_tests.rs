use super::durable_state_publication_tests::published_fixture;
use crate::agent::claude::generation::adversarial_test_support::prepare_durable_state_for_test;
use crate::agent::{AgentBinding, AgentBindingStore, AgentState};
use anyhow::ensure;
use std::fs;

const STALE_AUTHORITY_REJECTED: &str = "stale authority writer was not rejected";
const STALE_BINDING_REJECTED: &str = "stale binding writer was not rejected";

#[test]
fn stale_prepared_writer_cannot_overwrite_a_newer_pending_transition() {
    let fixture = published_fixture();
    let binding_path = fixture.binding_path();
    let binding_store = AgentBindingStore::new(binding_path.clone());
    let (old_binding, old_identity) = fixture
        .lease
        .read_json_with_identity::<AgentBinding>(&binding_path)
        .unwrap();
    let old_set = fixture.store.load_set(&fixture.lease).unwrap().unwrap();
    prepare_durable_state_for_test(&binding_path, AgentState::WaitingForInput).unwrap();
    let pending_set = fixture.store.load_set(&fixture.lease).unwrap().unwrap();
    let binding_bytes = fs::read(&binding_path).unwrap();

    let stale_authority =
        fixture
            .store
            .exchange_set_with_commit_fence(&fixture.lease, &old_set, &old_set, || Ok(()));
    assert!(stale_authority.is_err(), "{STALE_AUTHORITY_REJECTED}");
    assert_eq!(
        fixture.store.load_set(&fixture.lease).unwrap(),
        Some(pending_set.clone())
    );

    crate::paths::atomic_write_json(&binding_path, &old_binding).unwrap();
    let replacement_bytes = fs::read(&binding_path).unwrap();
    let (_, replacement_identity) = fixture
        .lease
        .read_json_with_identity::<AgentBinding>(&binding_path)
        .unwrap();
    assert_ne!(replacement_identity, old_identity);

    let mutation =
        super::super::mutation::GenerationMutation::acquire(&fixture.store, &fixture.lease)
            .unwrap();
    let stale_binding = mutation.write_path(
        &fixture.lease,
        &binding_path,
        Some(old_identity),
        &old_binding,
        || {
            let current = fixture.store.load_set(&fixture.lease)?.unwrap();
            ensure!(current == old_set, STALE_BINDING_REJECTED);
            Ok(())
        },
    );
    assert!(stale_binding.is_err(), "{STALE_BINDING_REJECTED}");
    assert_eq!(replacement_bytes, binding_bytes);
    assert_eq!(fs::read(&binding_path).unwrap(), replacement_bytes);
    let (_, current_identity) = fixture
        .lease
        .read_json_with_identity::<AgentBinding>(&binding_path)
        .unwrap();
    assert_eq!(current_identity, replacement_identity);
    assert_eq!(
        binding_store.read_raw_in_namespace(&fixture.lease).unwrap(),
        Some(old_binding)
    );
    assert_eq!(
        fixture.store.load_set(&fixture.lease).unwrap(),
        Some(pending_set)
    );
    let active = fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    assert_eq!(
        active,
        fixture
            .store
            .load_set(&fixture.lease)
            .unwrap()
            .unwrap()
            .active()
            .unwrap()
            .clone()
    );
}
