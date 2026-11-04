use super::support::*;
use crate::PendingFinalization;
use yolo_hub_identity::HubId;

const RECORD_FIELDS: usize = 3;
const REQUEST_FIELD: &str = "request";
const CREATION_FIELD: &str = "creation";

#[test]
fn persistence_round_trip_preserves_exact_finalization_without_writer_fields() {
    let record = pending(HubId::new());
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value.as_object().unwrap().len(), RECORD_FIELDS);
    assert_eq!(
        serde_json::from_value::<PendingFinalization>(value.clone()).unwrap(),
        record
    );
    let mut injected = value.clone();
    injected[UNKNOWN_FIELD] = true.into();
    assert!(serde_json::from_value::<PendingFinalization>(injected).is_err());
    let mut invalid = value.clone();
    invalid[REQUEST_FIELD][TOKEN_FIELD] = uuid::Uuid::nil().to_string().into();
    assert!(serde_json::from_value::<PendingFinalization>(invalid).is_err());
    let mut invalid = value;
    invalid[CREATION_FIELD][CAPABILITY_FIELD] = uuid::Uuid::nil().to_string().into();
    assert!(serde_json::from_value::<PendingFinalization>(invalid).is_err());
}

#[test]
fn diagnostic_format_never_prints_creation_token_capability_or_working_directory() {
    let record = pending(HubId::new());
    let debug = format!("{record:?}");
    for private in [
        record.request.fields().creation_token.to_string(),
        record.creation.creation_capability.as_uuid().to_string(),
        REQUEST_DIRECTORY.to_owned(),
    ] {
        assert!(!debug.contains(&private));
    }
    assert!(debug.contains(&record.creation.address.terminal_id.to_string()));
}
