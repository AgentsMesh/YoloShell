use crate::{constants::*, fixture::*, wire::*};
use serde_json::json;
use yolo_hub_client_access_protocol::*;

#[test]
fn complete_and_empty_configuration_round_trip_without_changing_the_entry_hub() {
    let mut original = configuration();
    original.clients.push(grant(SECOND_INDEX));
    let encoded = serde_json::to_vec(&original).unwrap();
    assert_eq!(
        serde_json::from_slice::<AccessConfiguration>(&encoded).unwrap(),
        original
    );
    original.clients.clear();
    assert_eq!(
        serde_json::from_value::<AccessConfiguration>(value(&original)).unwrap(),
        original
    );
}

#[test]
fn wire_schema_revision_and_entry_hub_are_validated() {
    let original = configuration();
    for (field, invalid) in [
        (FIELD_SCHEMA, json!(INVALID_SCHEMA)),
        (FIELD_SCHEMA, json!(FUTURE_SCHEMA)),
        (FIELD_REVISION, json!(INVALID_REVISION)),
        (FIELD_ENTRY_HUB, json!(uuid::Uuid::nil())),
        (
            FIELD_ENTRY_HUB,
            json!(original.entry_hub.to_string().to_uppercase()),
        ),
    ] {
        let mut candidate = value(&original);
        candidate[field] = invalid;
        rejected::<AccessConfiguration>(candidate);
    }
}

#[test]
fn duplicate_client_id_or_key_cannot_survive_configuration_deserialization() {
    let original = configuration();
    let mut id = grant(SECOND_INDEX);
    id.identity.client_id = original.clients[FIRST_INDEX].identity.client_id;
    let mut key = grant(SECOND_INDEX);
    key.identity.public_key = original.clients[FIRST_INDEX].identity.public_key;
    for duplicate in [id, key] {
        let mut candidate = original.clone();
        candidate.clients.push(duplicate);
        rejected::<AccessConfiguration>(value(&candidate));
    }
}

#[test]
fn configuration_rejects_invalid_nested_grants_and_client_capacity_overflow() {
    let mut original = configuration();
    original.clients = (FIRST_INDEX..MAX_CLIENTS).map(grant).collect();
    assert_eq!(
        serde_json::from_value::<AccessConfiguration>(value(&original)).unwrap(),
        original
    );
    original.clients.push(grant(MAX_CLIENTS));
    rejected::<AccessConfiguration>(value(&original));
    let mut invalid = configuration();
    invalid.clients[FIRST_INDEX].origins.clear();
    rejected::<AccessConfiguration>(value(&invalid));
}

#[test]
fn configuration_fields_are_required_unique_and_closed() {
    let original = configuration();
    for field in [FIELD_SCHEMA, FIELD_ENTRY_HUB, FIELD_REVISION, FIELD_CLIENTS] {
        let mut candidate = value(&original);
        candidate.as_object_mut().unwrap().remove(field);
        rejected::<AccessConfiguration>(candidate);
        assert!(
            serde_json::from_str::<AccessConfiguration>(&duplicate_field(&original, field))
                .is_err()
        );
    }
    let mut candidate = value(&original);
    candidate[UNKNOWN_FIELD] = json!(true);
    rejected::<AccessConfiguration>(candidate);
}
