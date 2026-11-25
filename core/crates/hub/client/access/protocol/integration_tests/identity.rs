use crate::{constants::*, fixture::*, wire::*};
use serde_json::json;
use yolo_hub_client_access_protocol::*;

#[test]
fn identity_wire_round_trip_preserves_exact_id_and_key() {
    let original = identity(FIRST_INDEX);
    let encoded = serde_json::to_string(&original).unwrap();
    assert_eq!(
        serde_json::from_str::<ClientIdentity>(&encoded).unwrap(),
        original
    );
    assert_eq!(
        value(&original)[FIELD_CLIENT_ID],
        original.client_id.to_string()
    );
}

#[test]
fn nil_and_noncanonical_client_uuids_are_rejected() {
    let original = identity(FIRST_INDEX);
    let id = original.client_id;
    for altered in [
        uuid::Uuid::nil().to_string(),
        id.simple().to_string(),
        id.hyphenated().to_string().to_uppercase(),
        id.urn().to_string(),
        id.braced().to_string(),
    ] {
        let mut candidate = value(&original);
        candidate[FIELD_CLIENT_ID] = json!(altered);
        rejected::<ClientIdentity>(candidate);
    }
}

#[test]
fn zero_truncated_oversized_and_out_of_range_keys_are_rejected() {
    let original = identity(FIRST_INDEX);
    for key in [
        json!(vec![u8::MIN; ED25519_PUBLIC_KEY_BYTES]),
        json!(vec![KEY_FILL; ED25519_PUBLIC_KEY_BYTES - ONE]),
        json!(vec![KEY_FILL; ED25519_PUBLIC_KEY_BYTES + ONE]),
        json!(vec![NEGATIVE_NUMBER; ED25519_PUBLIC_KEY_BYTES]),
    ] {
        let mut candidate = value(&original);
        candidate[FIELD_PUBLIC_KEY] = key;
        rejected::<ClientIdentity>(candidate);
    }
}

#[test]
fn identity_fields_are_required_unique_and_closed() {
    let original = identity(FIRST_INDEX);
    for field in [FIELD_CLIENT_ID, FIELD_PUBLIC_KEY] {
        let mut candidate = value(&original);
        candidate.as_object_mut().unwrap().remove(field);
        rejected::<ClientIdentity>(candidate);
        assert!(
            serde_json::from_str::<ClientIdentity>(&duplicate_field(&original, field)).is_err()
        );
    }
    let mut candidate = value(&original);
    candidate[UNKNOWN_FIELD] = json!(true);
    rejected::<ClientIdentity>(candidate);
}
