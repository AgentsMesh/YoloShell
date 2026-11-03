use super::support::*;
use crate::StoredState;
use yolo_hub_identity::HubId;

#[test]
fn seed_requires_exact_agent_and_terminal_origin_and_non_nil_registration() {
    let fixture = Fixture::new();
    let seed = fixture.seed();
    seed.validate(fixture.identity).unwrap();
    let mut invalid = seed.clone();
    invalid.request_id = uuid::Uuid::nil();
    assert!(invalid.validate(fixture.identity).is_err());
    let mut invalid = seed.clone();
    invalid.agent.hub_id = HubId::new();
    assert!(invalid.validate(fixture.identity).is_err());
    let mut invalid = seed.clone();
    invalid.scope.terminal.address.hub_id = HubId::new();
    assert!(invalid.validate(fixture.identity).is_err());
    State::initial(&seed).validate(&seed).unwrap();
    assert!(State::initial(&fixture.seed()).validate(&seed).is_err());
}

#[test]
fn seed_digest_covers_credential_scope_identity_and_request() {
    let fixture = Fixture::new();
    let seed = fixture.seed();
    let expected = seed.digest().unwrap();
    assert_eq!(seed.clone().digest().unwrap(), expected);
    let mut changed = seed.clone();
    changed.credential = yolo_hub_connector_protocol::ConnectorCredential::generate();
    assert_ne!(changed.digest().unwrap(), expected);
    changed = seed.clone();
    changed.request_id = uuid::Uuid::new_v4();
    assert_ne!(changed.digest().unwrap(), expected);
    changed = seed.clone();
    changed.scope.terminal.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
    assert_ne!(changed.digest().unwrap(), expected);
    let value = serde_json::to_value(&seed).unwrap();
    let restored: crate::Seed = serde_json::from_value(value).unwrap();
    assert_eq!(restored, seed);
}

#[test]
fn serialized_seed_requires_canonical_non_nil_request_identity() {
    let fixture = Fixture::new();
    let seed = fixture.seed();
    for invalid in [
        uuid::Uuid::nil().to_string(),
        seed.request_id.simple().to_string(),
    ] {
        let mut value = serde_json::to_value(&seed).unwrap();
        value[REQUEST_ID_FIELD] = invalid.into();
        assert!(serde_json::from_value::<crate::Seed>(value).is_err());
    }
}
