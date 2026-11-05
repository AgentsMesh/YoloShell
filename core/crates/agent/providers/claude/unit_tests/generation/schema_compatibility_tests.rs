use super::super::authority_set::ClaudeInvocationAuthorities;
use super::support::Fixture;

#[test]
fn authority_set_without_pending_transition_remains_loadable() {
    let fixture = Fixture::new();
    let authority = fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let encoded = std::fs::read_to_string(fixture.store.path()).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    let object = value.as_object_mut().unwrap();
    assert!(object.remove("pending_binding_transition").is_none());

    let loaded: ClaudeInvocationAuthorities = serde_json::from_value(value).unwrap();

    loaded.validate().unwrap();
    assert_eq!(loaded.active().unwrap(), &authority);
    assert!(loaded.pending_binding_transition.is_none());
}

#[test]
fn authority_without_termination_candidate_fails_closed() {
    let fixture = Fixture::new();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let encoded = std::fs::read_to_string(fixture.store.path()).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    value["authorities"][0]
        .as_object_mut()
        .unwrap()
        .remove("termination_candidate");

    assert!(serde_json::from_value::<ClaudeInvocationAuthorities>(value).is_err());
}
