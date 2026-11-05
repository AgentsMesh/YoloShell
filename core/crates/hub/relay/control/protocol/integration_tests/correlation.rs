use crate::fixtures::*;
use yolo_hub_agent_protocol::{AgentControlRequest, ObservationCursor};
use yolo_hub_relay_control_protocol::{
    Outcome, ProtocolError, Request, Response, decode_response, encode_response, field,
};

#[test]
fn a_delayed_reply_cannot_complete_a_new_launch_runtime_or_semantic_precondition() {
    let original = request();
    let original_reply = response(&original);
    let mut variants = Vec::new();
    let mut changed = original.fields().clone();
    changed.launch_id = alternate_uuid();
    variants.push(changed);
    let mut changed = original.fields().clone();
    changed.terminal.runtime_epoch = ALTERNATE_ID.parse().unwrap();
    variants.push(changed);
    let mut changed = original.fields().clone();
    changed.terminal.address.terminal_id = ALTERNATE_ID.parse().unwrap();
    variants.push(changed);
    for change in [
        ControlChange::Request,
        ControlChange::Stream,
        ControlChange::Sequence,
        ControlChange::Agent,
        ControlChange::Execution,
        ControlChange::Origin,
    ] {
        let mut changed = original.fields().clone();
        let mut control = changed.request.fields().clone();
        let mut stream = control.expected_cursor.stream();
        let mut sequence = control.expected_cursor.sequence();
        match change {
            ControlChange::Request => control.request_id = alternate_uuid(),
            ControlChange::Stream => stream = alternate_uuid(),
            ControlChange::Sequence => sequence += 1,
            ControlChange::Agent => control.target.address.agent_id = ALTERNATE_ID.parse().unwrap(),
            ControlChange::Execution => control.target.execution_id = ALTERNATE_ID.parse().unwrap(),
            ControlChange::Origin => {
                control.target.address.hub_id = ALTERNATE_ID.parse().unwrap();
                changed.terminal.address.hub_id = control.target.address.hub_id;
            }
        }
        control.expected_cursor = ObservationCursor::new(control.target, stream, sequence).unwrap();
        changed.request = AgentControlRequest::new(control).unwrap();
        variants.push(changed);
    }
    for fields in variants {
        let newer = Request::new(fields).unwrap();
        assert_eq!(
            original_reply.validate_for(&newer),
            Err(ProtocolError::ResponseMismatch)
        );
        let reply = Response::new(&newer, Outcome::Unknown {}).unwrap();
        let reply = decode_response(&encode_response(&reply).unwrap()).unwrap();
        assert_eq!(
            reply.validate_for(&original),
            Err(ProtocolError::ResponseMismatch)
        );
        assert_eq!(reply.validate_for(&newer), Ok(()));
    }
}

enum ControlChange {
    Request,
    Stream,
    Sequence,
    Agent,
    Execution,
    Origin,
}

#[test]
fn decoding_a_receipt_alone_still_rejects_intrinsic_identity_mismatch() {
    let request = request();
    let reply = response(&request);
    for path in [path::RECEIPT_ID, path::RECEIPT_EXECUTION] {
        let payload = modified_json(&reply, path, ALTERNATE_ID.into());
        assert_eq!(decode_response(&payload), Err(ProtocolError::InvalidFrame));
    }
}

#[test]
fn private_frames_have_exact_fields_and_never_carry_writer_authority() {
    let request = request();
    let value = serde_json::to_value(&request).unwrap();
    let keys: Vec<_> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            field::LAUNCH_ID,
            field::REQUEST,
            field::SCHEMA_VERSION,
            field::TERMINAL
        ]
    );
    let value = serde_json::to_value(response(&request)).unwrap();
    let keys: Vec<_> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            field::LAUNCH_ID,
            field::OUTCOME,
            field::REQUEST,
            field::SCHEMA_VERSION,
            field::TERMINAL
        ]
    );
    assert_eq!(value[field::OUTCOME][field::OUTCOME], RECEIPT_OUTCOME);
    let keys: Vec<_> = value[field::REQUEST]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            CONTROL_ACTION_FIELD,
            CONTROL_CURSOR_FIELD,
            yolo_hub_agent_protocol::REQUEST_ID_FIELD,
            field::SCHEMA_VERSION,
            CONTROL_TARGET_FIELD
        ]
    );
}
