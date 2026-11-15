use uuid::Uuid;
use yolo_capnp_value::Value;
use yolo_protocol_validation::{validate_attachment_event, validate_event, validate_response};
use yolo_terminal_model::OutputChunk;
use yolo_wire::{
    EventEnvelope, PROTOCOL_VERSION, RequestEnvelope, ResponseEnvelope, event, method,
};

const REQUEST_ID: &str = "protocol-validation-request";
const SESSION_ID: u128 = 0x1200;
const RUNTIME_EPOCH: u128 = 0x3400;
const OUTPUT_SEQUENCE: u64 = 7;
const OUTPUT_BYTES: &[u8] = b"output";
const VERSION_INCREMENT: u16 = 1;
const OTHER_RUNTIME_EPOCH: u128 = RUNTIME_EPOCH + 1;

#[test]
fn response_validation_binds_the_request_identity() {
    let request = RequestEnvelope::new(REQUEST_ID, method::HELLO, Value::Null);
    let response = ResponseEnvelope::empty(&request);
    assert!(validate_response(&request, &response).is_ok());

    let mut changed = response;
    changed.version = PROTOCOL_VERSION.saturating_add(VERSION_INCREMENT);
    assert!(validate_response(&request, &changed).is_err());
}

#[test]
fn attachment_validation_binds_envelope_and_payload_epochs() {
    let terminal_id = Uuid::from_u128(SESSION_ID);
    let runtime_epoch = Uuid::from_u128(RUNTIME_EPOCH);
    let chunk = OutputChunk {
        runtime_epoch,
        sequence: OUTPUT_SEQUENCE,
        data: OUTPUT_BYTES.to_vec(),
    };
    let mut envelope = EventEnvelope::new(
        event::TERMINAL_OUTPUT,
        Some(terminal_id.to_string()),
        Some(runtime_epoch.to_string()),
        &chunk,
    )
    .unwrap();
    assert!(validate_event(&envelope).is_ok());
    assert!(validate_attachment_event(&envelope, terminal_id, runtime_epoch).is_ok());

    envelope.runtime_epoch = Some(Uuid::from_u128(OTHER_RUNTIME_EPOCH).to_string());
    assert!(validate_attachment_event(&envelope, terminal_id, runtime_epoch).is_err());
}
