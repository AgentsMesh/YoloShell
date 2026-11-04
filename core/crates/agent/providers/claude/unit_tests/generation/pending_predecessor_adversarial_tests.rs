use super::durable_state_publication_tests::published_fixture;
use super::pending_predecessor_adversarial_test_support::{
    PublicationMutation, add_publication_marker, increment_predecessor_publication_generation,
    increment_publication_authority_revision, increment_publication_generation,
    remove_publication_marker, replace_publication_invocation,
};
use crate::agent::AgentState;
use crate::agent::claude::generation::adversarial_test_support::prepare_durable_state_for_test;
use crate::agent::claude::generation::authority_set::ClaudeInvocationAuthorities;
use crate::agent::claude::generation::{AuthorityProcessIdentity, ClaudeGenerationAuthority};
use uuid::Uuid;

const IDENTITY_INCREMENT: u64 = 1;
const FIELD_SUFFIX: &str = "-forged";

#[test]
fn every_predecessor_authority_identity_field_is_bound_to_the_target() {
    let mutations: &[fn(&mut ClaudeGenerationAuthority)] = &[
        |value| value.runtime_epoch = Uuid::new_v4(),
        |value| value.invocation_id = Uuid::new_v4(),
        |value| value.hook_token = Uuid::new_v4(),
        |value| value.helper_capability = Uuid::new_v4(),
        |value| increment_identity(&mut value.worker),
        |value| increment_identity(&mut value.process),
        |value| value.handshake_boot_id.push_str(FIELD_SUFFIX),
        |value| value.handshake_deadline_ns += IDENTITY_INCREMENT,
        |value| value.provider_session_id = Uuid::new_v4().hyphenated().to_string(),
        |value| value.discover_session = !value.discover_session,
        |value| value.cwd.push(FIELD_SUFFIX),
        |value| value.data_home.push(FIELD_SUFFIX),
        |value| value.revision += IDENTITY_INCREMENT,
        |value| value.generation += IDENTITY_INCREMENT,
        |value| value.last_event_sequence += IDENTITY_INCREMENT,
    ];
    for mutate in mutations {
        assert_predecessor_mutation_rejected(*mutate);
    }
}

#[test]
fn predecessor_binding_revision_and_transcript_are_bound_to_the_target() {
    for mutate in [
        (|value: &mut ClaudeGenerationAuthority| value.binding_revision = None)
            as fn(&mut ClaudeGenerationAuthority),
        |value| value.transcript_path = None,
        |value| value.transcript_identity = None,
        |value| value.state = AgentState::Exited,
    ] {
        assert_predecessor_mutation_rejected(mutate);
    }
}

#[test]
fn published_predecessor_marker_fields_are_bound_to_the_transition() {
    for mutate in [
        remove_publication_marker as fn(&mut serde_json::Map<String, serde_json::Value>),
        replace_publication_invocation,
        increment_predecessor_publication_generation,
        increment_publication_generation,
        increment_publication_authority_revision,
    ] {
        assert_publication_mutation_rejected(false, mutate);
    }
}

#[test]
fn revoked_predecessor_cannot_gain_a_publication_marker() {
    assert_publication_mutation_rejected(true, add_publication_marker);
}

fn assert_predecessor_mutation_rejected(mutate: fn(&mut ClaudeGenerationAuthority)) {
    let fixture = published_fixture();
    prepare_durable_state_for_test(&fixture.binding_path(), AgentState::WaitingForInput).unwrap();
    let mut forged: serde_json::Value = fixture.lease.read_json(fixture.store.path()).unwrap();
    let transition = forged
        .get_mut("pending_binding_transition")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap();
    let predecessor = transition.get_mut("predecessor_authority").unwrap();
    let mut authority: ClaudeGenerationAuthority =
        serde_json::from_value(predecessor.clone()).unwrap();
    mutate(&mut authority);
    *predecessor = serde_json::to_value(authority).unwrap();
    fixture
        .lease
        .write_json(fixture.store.path(), &forged)
        .unwrap();
    assert!(fixture.store.load_in_namespace(&fixture.lease).is_err());
    assert!(
        fixture
            .lease
            .read_json::<ClaudeInvocationAuthorities>(fixture.store.path())
            .is_ok()
    );
}

fn assert_publication_mutation_rejected(revoked: bool, mutate: PublicationMutation) {
    let fixture = published_fixture();
    let committed = fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    if revoked {
        fixture
            .store
            .fence_binding_if_active_in_namespace(
                &fixture.lease,
                fixture.authentication(),
                &committed,
            )
            .unwrap();
    }
    prepare_durable_state_for_test(&fixture.binding_path(), AgentState::WaitingForInput).unwrap();
    let mut forged: serde_json::Value = fixture.lease.read_json(fixture.store.path()).unwrap();
    let transition = forged
        .get_mut("pending_binding_transition")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap();
    mutate(transition);
    fixture
        .lease
        .write_json(fixture.store.path(), &forged)
        .unwrap();
    assert!(fixture.store.load_in_namespace(&fixture.lease).is_err());
}

fn increment_identity(identity: &mut AuthorityProcessIdentity) {
    identity.start_time += IDENTITY_INCREMENT;
}
