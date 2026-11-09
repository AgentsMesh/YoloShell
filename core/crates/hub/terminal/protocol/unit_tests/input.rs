use crate::fixtures::{self, constants::*, roundtrip};
use uuid::Uuid;
use yolo_hub_terminal_protocol::*;

#[test]
fn input_preserves_raw_control_bytes_with_exact_runtime_lease() {
    let mut fields = fixtures::input_fields();
    fields.data = CONTROL_BYTES.to_vec();
    let request = InputRequest::new(fields).unwrap();
    assert_eq!(request.fields().data, CONTROL_BYTES);
    assert_eq!(request.fields().grant.fields().target, fixtures::target());
    roundtrip(&request);
    let fields: InputFields = request.into();
    assert_eq!(fields.data, CONTROL_BYTES);
}

#[test]
fn input_payload_limits_are_enforced() {
    let mut fields = fixtures::input_fields();
    fields.request_id = Uuid::nil();
    assert_eq!(InputRequest::new(fields), Err(PublicError::InvalidRequest));
    for length in [0, MAX_INPUT_BYTES + STEP as usize] {
        let mut fields = fixtures::input_fields();
        fields.data = vec![0; length];
        assert_eq!(InputRequest::new(fields), Err(PublicError::InvalidPayload));
    }
    let mut fields = fixtures::input_fields();
    fields.data = vec![0; MAX_INPUT_BYTES];
    roundtrip(&InputRequest::new(fields).unwrap());
}

#[test]
fn resize_requires_same_writer_authority_as_input() {
    let mut request = ResizeRequest {
        request_id: Uuid::from_u128(REQUEST_ID),
        grant: LeaseGrant::new(fixtures::grant_fields()).unwrap(),
        size: TerminalSize::default(),
    };
    assert_eq!(request.validate(), Ok(()));
    roundtrip(&request);
    request.request_id = Uuid::nil();
    assert_eq!(request.validate(), Err(PublicError::InvalidRequest));
}
