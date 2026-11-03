use crate::constants::{CANONICAL_UUID, NONCANONICAL_UUID, UUID_FIELD_NAME};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
struct Fixture {
    #[serde(with = "yolo_crypto::serde_uuid")]
    id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
struct OptionalFixture {
    #[serde(with = "yolo_crypto::serde_uuid::option")]
    id: Option<Uuid>,
}

#[test]
fn canonical_uuid_strings_round_trip() {
    let id = Uuid::parse_str(CANONICAL_UUID).unwrap();
    let encoded = serde_json::to_string(&Fixture { id }).unwrap();
    assert_eq!(serde_json::from_str::<Fixture>(&encoded).unwrap().id, id);
    assert_eq!(
        yolo_crypto::serde_uuid::parse_canonical(CANONICAL_UUID),
        Some(id)
    );
}

#[test]
fn byte_arrays_and_noncanonical_strings_are_rejected() {
    let id = Uuid::parse_str(CANONICAL_UUID).unwrap();
    let invalid_values = [
        serde_json::Value::Array(
            id.as_bytes()
                .iter()
                .copied()
                .map(serde_json::Value::from)
                .collect(),
        ),
        serde_json::Value::String(NONCANONICAL_UUID.to_owned()),
    ];
    for invalid in invalid_values {
        let encoded = serde_json::Value::Object(
            [(UUID_FIELD_NAME.to_owned(), invalid)]
                .into_iter()
                .collect(),
        );
        assert!(serde_json::from_value::<Fixture>(encoded).is_err());
    }
    assert!(yolo_crypto::serde_uuid::parse_canonical(NONCANONICAL_UUID).is_none());
}

#[test]
fn optional_uuid_adapter_preserves_some_and_none() {
    let id = Uuid::parse_str(CANONICAL_UUID).unwrap();
    for fixture in [
        OptionalFixture { id: Some(id) },
        OptionalFixture { id: None },
    ] {
        let encoded = serde_json::to_string(&fixture).unwrap();
        let decoded = serde_json::from_str::<OptionalFixture>(&encoded).unwrap();
        assert_eq!(decoded.id, fixture.id);
    }
}
