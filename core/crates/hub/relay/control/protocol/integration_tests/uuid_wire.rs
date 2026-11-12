use crate::fixtures::*;
use yolo_hub_relay_control_protocol::{ProtocolError, decode_request, decode_response};

fn invalid_spellings() -> Vec<String> {
    vec![
        ALTERNATE_ID.to_uppercase(),
        ALTERNATE_ID.replace('-', ""),
        format!("{{{ALTERNATE_ID}}}"),
        format!("urn:uuid:{ALTERNATE_ID}"),
        NIL_ID.into(),
    ]
}

#[test]
fn every_request_uuid_is_canonical_non_nil_even_in_public_dtos() {
    let request = request();
    for path in [
        path::LAUNCH,
        path::REQUEST_ID,
        path::CURSOR_STREAM,
        path::TERMINAL_HUB,
        path::SESSION,
        path::EPOCH,
        path::AGENT,
        path::EXECUTION,
    ] {
        for spelling in invalid_spellings() {
            let payload = modified_json(&request, path, spelling.into());
            assert_eq!(decode_request(&payload), Err(ProtocolError::InvalidFrame));
        }
    }
}

#[test]
fn response_echo_and_receipt_uuids_have_the_same_strict_spelling() {
    let reply = response(&request());
    for path in [
        path::LAUNCH,
        path::REQUEST_ID,
        path::CURSOR_STREAM,
        path::RECEIPT_ID,
        path::RECEIPT_EXECUTION,
    ] {
        for spelling in invalid_spellings() {
            let payload = modified_json(&reply, path, spelling.into());
            assert_eq!(decode_response(&payload), Err(ProtocolError::InvalidFrame));
        }
    }
}

#[test]
fn canonical_alphabetic_uuid_is_accepted_when_all_echoes_match() {
    let mut fields = fields();
    fields.launch_id = alternate_uuid();
    let request = yolo_hub_relay_control_protocol::Request::new(fields).unwrap();
    let payload = yolo_hub_relay_control_protocol::encode_request(&request).unwrap();
    assert_eq!(decode_request(&payload), Ok(request));
}
