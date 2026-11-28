use crate::fixtures::{self, constants::*, reject_field, roundtrip};
use uuid::Uuid;
use yolo_hub_terminal_protocol::*;

#[test]
fn shell_creation_retains_independent_durable_identity() {
    let request = CreateShellRequest::new(fixtures::create_fields()).unwrap();
    assert_ne!(request.fields().request_id, request.fields().creation_token);
    roundtrip(&request);
    let mut fields: CreateShellFields = request.clone().into();
    fields.request_id = Uuid::new_v4();
    fields.name = None;
    let retry = CreateShellRequest::new(fields).unwrap();
    assert_eq!(
        retry.fields().creation_token,
        request.fields().creation_token
    );
    roundtrip(&retry);
}

#[test]
fn malformed_creation_metadata_is_rejected() {
    let mut fields = fixtures::create_fields();
    fields.schema_version += STEP as u16;
    assert_eq!(
        CreateShellRequest::new(fields),
        Err(PublicError::UnsupportedSchema)
    );
    for invalid in INVALID_TEXT {
        let mut fields = fixtures::create_fields();
        fields.cwd = invalid.into();
        assert_eq!(
            CreateShellRequest::new(fields),
            Err(PublicError::InvalidMetadata)
        );
        let mut fields = fixtures::create_fields();
        fields.name = Some(invalid.into());
        assert_eq!(
            CreateShellRequest::new(fields),
            Err(PublicError::InvalidMetadata)
        );
    }
    let mut fields = fixtures::create_fields();
    fields.cwd = NAME.repeat(MAX_DIRECTORY_BYTES + STEP as usize);
    assert!(CreateShellRequest::new(fields).is_err());
    let mut fields = fixtures::create_fields();
    fields.name = Some(NAME.repeat(MAX_TERMINAL_NAME_BYTES + STEP as usize));
    assert!(CreateShellRequest::new(fields).is_err());
}

#[test]
fn creation_ids_are_checked_in_memory_and_on_wire() {
    let request = CreateShellRequest::new(fixtures::create_fields()).unwrap();
    for field in [field::REQUEST_ID, field::CREATION_TOKEN] {
        for invalid in [NIL_ID, INVALID_ID, CAPABILITY_UPPER, CAPABILITY_COMPACT] {
            reject_field(&request, field, invalid.into());
        }
    }
    let mut fields = fixtures::create_fields();
    fields.request_id = Uuid::nil();
    assert_eq!(
        CreateShellRequest::new(fields),
        Err(PublicError::InvalidRequest)
    );
    let mut fields = fixtures::create_fields();
    fields.creation_token = Uuid::nil();
    assert_eq!(
        CreateShellRequest::new(fields),
        Err(PublicError::InvalidRequest)
    );
}

#[test]
fn creation_result_redacts_capability_in_aggregate_debug() {
    let result = CreateShellResult {
        address: fixtures::address(),
        creation_capability: SecretCapability::generate(),
    };
    roundtrip(&result);
    let debug = format!("{result:?}");
    assert!(debug.contains(REDACTED));
    assert!(!debug.contains(&result.creation_capability.as_uuid().to_string()));
}

#[test]
fn cleanup_outcomes_roundtrip_without_normalizing_unknown_results() {
    for (outcome, text) in [
        (CleanupTerminationOutcome::Terminated, CLEANUP_TERMINATED),
        (CleanupTerminationOutcome::Survivor, CLEANUP_SURVIVOR),
    ] {
        roundtrip(&outcome);
        assert_eq!(
            serde_json::to_value(outcome).unwrap(),
            serde_json::Value::String(text.into())
        );
    }
    for invalid in [
        serde_json::Value::String(UNKNOWN_VALUE.into()),
        serde_json::Value::Bool(true),
        serde_json::Value::Null,
        serde_json::Value::Object(serde_json::Map::new()),
    ] {
        assert!(serde_json::from_value::<CleanupTerminationOutcome>(invalid).is_err());
    }
}
