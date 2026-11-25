use crate::fixtures::*;
use yolo_hub_relay_control_protocol::{
    NotSentReason, Outcome, ProtocolError, Response, decode_request, decode_response,
    encode_request, encode_response,
};

#[test]
fn unknown_fields_are_rejected_at_every_request_boundary() {
    let request = request();
    for path in [
        "",
        "/terminal",
        "/terminal/address",
        "/request",
        "/request/target",
        "/request/target/address",
        "/request/expected_cursor",
        "/request/action",
    ] {
        assert_eq!(
            decode_request(&extra_field(&request, path)),
            Err(ProtocolError::InvalidFrame)
        );
    }
}

#[test]
fn response_outcomes_and_receipts_are_closed_schemas() {
    let request = request();
    let reply = response(&request);
    for path in [
        "",
        "/outcome",
        "/outcome/receipt",
        "/outcome/receipt/target",
        "/outcome/receipt/target/address",
        "/outcome/receipt/outcome",
        "/request",
    ] {
        assert_eq!(
            decode_response(&extra_field(&reply, path)),
            Err(ProtocolError::InvalidFrame)
        );
    }
    for outcome in [
        Outcome::Unknown {},
        Outcome::NotSent {
            reason: NotSentReason::NoWriter,
        },
    ] {
        let reply = Response::new(&request, outcome).unwrap();
        assert_eq!(
            decode_response(&extra_field(&reply, "/outcome")),
            Err(ProtocolError::InvalidFrame)
        );
    }
    let reply = Response::new(
        &request,
        Outcome::NotSent {
            reason: NotSentReason::NoWriter,
        },
    )
    .unwrap();
    for path in ["/outcome/outcome", "/outcome/reason"] {
        assert_eq!(
            decode_response(&modified_json(&reply, path, EXTRA_VALUE.into())),
            Err(ProtocolError::InvalidFrame)
        );
    }
}

#[test]
fn duplicate_keys_are_rejected_without_json_value_normalization() {
    let request = request();
    let encoded = serde_json::to_string(&request).unwrap();
    for (needle, replacement) in [
        (
            "\"schema_version\":2",
            "\"schema_version\":2,\"schema_version\":2",
        ),
        (
            "\"schema_version\":2",
            "\"schema_version\":2,\"schema_version\":1",
        ),
        ("\"sequence\":1", "\"sequence\":1,\"sequence\":1"),
        (
            "\"operation\":\"interrupt\"",
            "\"operation\":\"interrupt\",\"operation\":\"interrupt\"",
        ),
    ] {
        let payload = encoded.replace(needle, replacement);
        assert_ne!(payload, encoded);
        assert_eq!(
            decode_request(payload.as_bytes()),
            Err(ProtocolError::InvalidFrame)
        );
    }
    let encoded = serde_json::to_string(&response(&request)).unwrap();
    for (needle, replacement) in [
        (
            "\"outcome\":\"receipt\"",
            "\"outcome\":\"receipt\",\"outcome\":\"receipt\"",
        ),
        (
            "\"state\":\"delivered\"",
            "\"state\":\"delivered\",\"state\":\"unknown\"",
        ),
    ] {
        let payload = encoded.replace(needle, replacement);
        assert_ne!(payload, encoded);
        assert_eq!(
            decode_response(payload.as_bytes()),
            Err(ProtocolError::InvalidFrame)
        );
    }
}

#[test]
fn nested_semantic_invariants_apply_during_private_decode() {
    let request = request();
    for (path, replacement) in [
        (path::SCHEMA, UNSUPPORTED_SCHEMA.into()),
        (path::REQUEST_SCHEMA, UNSUPPORTED_SCHEMA.into()),
        (path::CURSOR_SEQUENCE, INVALID_SEQUENCE.into()),
        (path::CURSOR_EXECUTION, ALTERNATE_ID.into()),
        (path::TERMINAL_HUB, ALTERNATE_ID.into()),
    ] {
        assert_eq!(
            decode_request(&modified_json(&request, path, replacement)),
            Err(ProtocolError::InvalidFrame)
        );
    }
}
