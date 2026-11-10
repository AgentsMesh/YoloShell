use crate::fixtures::*;
use std::error::Error;
use yolo_hub_relay_control_protocol::{ProtocolError, decode_request};

#[test]
fn errors_never_preserve_private_parser_input_or_a_source_chain() {
    let error = decode_request(PRIVATE_PAYLOAD.as_bytes()).unwrap_err();
    assert_eq!(error, ProtocolError::InvalidFrame);
    for error in [
        error,
        ProtocolError::UnsupportedSchema,
        ProtocolError::InvalidIdentity,
        ProtocolError::InvalidRequest,
        ProtocolError::UnsupportedAction,
        ProtocolError::OriginMismatch,
        ProtocolError::ReceiptMismatch,
        ProtocolError::ResponseMismatch,
        ProtocolError::EmptyFrame,
        ProtocolError::FrameTooLarge,
    ] {
        assert!(!error.to_string().contains(PRIVATE_PAYLOAD));
        assert!(!format!("{error:?}").contains(PRIVATE_PAYLOAD));
        assert!(!error.to_string().is_empty());
        assert_eq!(error.source().map(ToString::to_string), None);
    }
}
