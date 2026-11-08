use super::invocation_set_tests::parked_pair;
use super::support::{Fixture, binding};
use crate::agent::claude::generation::GenerationApplyDisposition;
use crate::agent::{AgentBindingStore, AgentState};

#[test]
fn foreground_b_to_a_reactivation_uses_the_durable_publisher() {
    let fixture = Fixture::new();
    let pair = parked_pair(&fixture);
    let binding_store = AgentBindingStore::new(fixture.binding_path());
    binding_store
        .save_authoritative_in_namespace(
            &fixture.lease,
            &binding(&pair.second),
            &pair.second,
            fixture.runtime(),
        )
        .unwrap();

    let (reactivated, disposition, changed) = fixture
        .store
        .transition_foreground_durable_in_namespace(
            &fixture.lease,
            pair.first_authentication,
            &pair.first,
            pair.first.last_event_sequence,
            &pair.first_event,
            pair.first.transcript_identity,
        )
        .unwrap();
    assert_eq!(disposition, GenerationApplyDisposition::Replay);
    assert!(changed);
    assert_eq!(reactivated.state, AgentState::Running);
    assert!(reactivated.binding_revision.is_none());

    let (committed, published) = fixture
        .store
        .publish_binding_state_in_namespace(
            &fixture.lease,
            &fixture.binding_path(),
            reactivated.authentication(),
            &reactivated,
            true,
            |_| Ok(()),
        )
        .unwrap();
    assert_eq!(committed.invocation_id, pair.first.invocation_id);
    assert!(committed.matches_binding(&published));
    assert_eq!(published.provider_session_id, pair.first.provider_session_id);
    assert_eq!(published.state, AgentState::Running);
    assert_eq!(
        binding_store.read_raw_in_namespace(&fixture.lease).unwrap(),
        Some(published)
    );
    assert_eq!(
        fixture.store.load_in_namespace(&fixture.lease).unwrap(),
        Some(committed)
    );
}
