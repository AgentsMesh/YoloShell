use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::*;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn correlated_protocol_rejections_are_known_and_internal_failure_is_uncertain() {
    let call = Call::new(HubId::new(), Command::Hello {}).unwrap();
    for (code, expected) in [
        (PARSE_ERROR, PublicError::InvalidRequest),
        (INVALID_REQUEST, PublicError::InvalidRequest),
        (METHOD_NOT_FOUND, PublicError::Unsupported),
        (INVALID_PARAMS, PublicError::InvalidPayload),
        (INTERNAL_ERROR, PublicError::OutcomeUnknown),
    ] {
        let answer = Answer::failure(Fault::new(Some(call.id.clone()), code));
        assert_eq!(answer.reply(&call), Ok(Reply::Error { code: expected }));
    }
}

#[test]
fn notifications_have_no_delivery_receipt_and_old_request_envelopes_are_not_notifications() {
    let notification = serde_json::json!({"jsonrpc":"2.0", "method":"create", "params":{}});
    assert!(Call::is_notification(&notification));
    for value in [
        serde_json::Value::Null,
        serde_json::json!({}),
        serde_json::json!({"jsonrpc":"1.0","method":"create"}),
        serde_json::json!({"jsonrpc":"2.0","method":true}),
        serde_json::json!({"jsonrpc":"2.0","method":"create","id":null}),
    ] {
        assert!(!Call::is_notification(&value));
    }
}
