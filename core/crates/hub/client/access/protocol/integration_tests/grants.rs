use crate::{constants::*, fixture::*, wire::*};
use serde_json::json;
use yolo_hub_client_access_protocol::*;
use yolo_hub_routing::Right;

#[test]
fn grant_round_trip_preserves_origins_rights_and_expiry() {
    let mut original = grant(FIRST_INDEX);
    original
        .origins
        .push(origin(SECOND_INDEX, Right::TerminalWrite));
    let decoded: ClientGrant = serde_json::from_value(value(&original)).unwrap();
    assert_eq!(decoded, original);
    assert!(decoded.allows(hub(SECOND_INDEX), &rights(Right::TerminalWrite)));
    assert!(!decoded.allows(hub(SECOND_INDEX), &rights(Right::Observe)));
    assert_eq!(
        decoded.validate_at(EXPIRES_AT_MS),
        Err(AccessError::Expired)
    );
}

#[test]
fn wire_grants_reject_empty_rights_origins_zero_expiry_and_duplicate_origins() {
    let original = grant(FIRST_INDEX);
    let mut no_rights = value(&original);
    no_rights[FIELD_ORIGINS][FIRST_INDEX][FIELD_RIGHTS] = json!([]);
    let mut no_origins = value(&original);
    no_origins[FIELD_ORIGINS] = json!([]);
    let mut no_expiry = value(&original);
    no_expiry[FIELD_EXPIRY] = json!(INVALID_EXPIRY_MS);
    let mut duplicate = value(&original);
    duplicate[FIELD_ORIGINS] =
        json!([original.origins[FIRST_INDEX], original.origins[FIRST_INDEX]]);
    for candidate in [no_rights, no_origins, no_expiry, duplicate] {
        rejected::<ClientGrant>(candidate);
    }
}

#[test]
fn wire_origin_limits_are_enforced_at_deserialization() {
    let mut original = grant(FIRST_INDEX);
    original.origins = (FIRST_INDEX..MAX_CLIENT_ORIGINS)
        .map(|index| origin(index, Right::Observe))
        .collect();
    assert_eq!(
        serde_json::from_value::<ClientGrant>(value(&original)).unwrap(),
        original
    );
    original
        .origins
        .push(origin(MAX_CLIENT_ORIGINS, Right::Observe));
    rejected::<ClientGrant>(value(&original));
}

#[test]
fn malformed_nested_identity_and_origin_cannot_enter_a_grant() {
    let original = grant(FIRST_INDEX);
    let mut identity = value(&original);
    identity[FIELD_IDENTITY][FIELD_CLIENT_ID] = json!(uuid::Uuid::nil());
    let mut origin = value(&original);
    origin[FIELD_ORIGINS][FIRST_INDEX][FIELD_ORIGIN] = json!(uuid::Uuid::nil());
    let mut rights = value(&original);
    rights[FIELD_ORIGINS][FIRST_INDEX][FIELD_RIGHTS] = json!([UNKNOWN_RIGHT]);
    for candidate in [identity, origin, rights] {
        rejected::<ClientGrant>(candidate);
    }
}

#[test]
fn grant_and_origin_fields_reject_unknown_missing_and_duplicate_values() {
    let original = grant(FIRST_INDEX);
    for field in [FIELD_IDENTITY, FIELD_ORIGINS, FIELD_EXPIRY] {
        let mut candidate = value(&original);
        candidate.as_object_mut().unwrap().remove(field);
        rejected::<ClientGrant>(candidate);
        assert!(serde_json::from_str::<ClientGrant>(&duplicate_field(&original, field)).is_err());
    }
    let mut candidate = value(&original);
    candidate[UNKNOWN_FIELD] = json!(true);
    rejected::<ClientGrant>(candidate);
    let origin = &original.origins[FIRST_INDEX];
    for field in [FIELD_ORIGIN, FIELD_RIGHTS] {
        let mut candidate = value(origin);
        candidate.as_object_mut().unwrap().remove(field);
        rejected::<OriginAccess>(candidate);
        assert!(serde_json::from_str::<OriginAccess>(&duplicate_field(origin, field)).is_err());
    }
    let mut candidate = value(origin);
    candidate[UNKNOWN_FIELD] = json!(true);
    rejected::<OriginAccess>(candidate);
}
