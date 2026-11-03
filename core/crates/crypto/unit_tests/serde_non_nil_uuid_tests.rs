use crate::constants::CANONICAL_UUID;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
struct Fixture {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    id: Uuid,
}

#[test]
fn non_nil_uuid_round_trips_canonically() {
    let id = Uuid::parse_str(CANONICAL_UUID).unwrap();
    let encoded = serde_json::to_string(&Fixture { id }).unwrap();
    assert_eq!(serde_json::from_str::<Fixture>(&encoded).unwrap().id, id);
}

#[test]
fn nil_uuid_is_rejected() {
    let encoded = serde_json::to_string(&Fixture { id: Uuid::nil() }).unwrap();
    assert!(serde_json::from_str::<Fixture>(&encoded).is_err());
}
