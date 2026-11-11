use crate::fixtures::{self, constants::*, reject_field, roundtrip};
use uuid::Uuid;
use yolo_hub_terminal_protocol::*;

#[test]
fn run_requests_keep_bounded_input_and_independent_identity() {
    let request = TerminalRunRequest::new(fixtures::run_fields()).unwrap();
    assert_ne!(request.fields().request_id, request.fields().creation_token);
    roundtrip(&request);
    let debug = format!("{request:?}");
    assert!(!debug.contains("secret-launch-command"));
    assert!(debug.contains("[REDACTED]"));
}

#[test]
fn run_requests_reject_empty_and_oversized_input() {
    let mut fields = fixtures::run_fields();
    fields.input.clear();
    assert_eq!(
        TerminalRunRequest::new(fields),
        Err(PublicError::InvalidPayload)
    );
    let mut fields = fixtures::run_fields();
    fields.input = vec![b'x'; MAX_INPUT_BYTES + 1];
    assert_eq!(
        TerminalRunRequest::new(fields),
        Err(PublicError::InvalidPayload)
    );
}

#[test]
fn run_requests_reject_bad_schema_ids_and_metadata() {
    let mut fields = fixtures::run_fields();
    fields.schema_version += STEP as u16;
    assert_eq!(
        TerminalRunRequest::new(fields),
        Err(PublicError::UnsupportedSchema)
    );
    let request = TerminalRunRequest::new(fixtures::run_fields()).unwrap();
    for member in [field::REQUEST_ID, field::CREATION_TOKEN] {
        for invalid in [NIL_ID, INVALID_ID, CAPABILITY_UPPER, CAPABILITY_COMPACT] {
            reject_field(&request, member, invalid.into());
        }
    }
    let mut fields = fixtures::run_fields();
    fields.request_id = Uuid::nil();
    assert_eq!(
        TerminalRunRequest::new(fields),
        Err(PublicError::InvalidRequest)
    );
    for invalid in INVALID_TEXT {
        let mut fields = fixtures::run_fields();
        fields.cwd = invalid.into();
        assert_eq!(
            TerminalRunRequest::new(fields),
            Err(PublicError::InvalidMetadata)
        );
    }
    let mut fields = fixtures::run_fields();
    fields.name = Some(NAME.repeat(MAX_TERMINAL_NAME_BYTES + STEP as usize));
    assert_eq!(
        TerminalRunRequest::new(fields),
        Err(PublicError::InvalidMetadata)
    );
}
