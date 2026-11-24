use super::fixtures::constants::*;
use crate::{ClientError, local::validate_uid};
use yolo_hub_jsonrpc_transport::TransportError;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn outcomes_and_error_sources_remain_distinguishable() {
    let known = [
        ClientError::Validation(PublicError::InvalidRequest),
        ClientError::Remote(PublicError::Unauthorized),
        ClientError::InvalidResponse(PublicError::InvalidPayload),
        ClientError::Transport(TransportError::Deadline),
        ClientError::Poisoned,
        ClientError::Unsupported,
        ClientError::WrongLocalUser,
    ];
    for error in known {
        assert!(!error.outcome_is_unknown());
        assert!(!error.to_string().is_empty());
    }
    let unknown = [
        ClientError::OutcomeUnknown(TransportError::Deadline),
        ClientError::OutcomeUnknownResponse(PublicError::InvalidRequest),
        ClientError::Remote(PublicError::OutcomeUnknown),
    ];
    for error in unknown {
        assert!(error.outcome_is_unknown());
        assert!(!error.to_string().is_empty());
    }
    let converted = ClientError::from(TransportError::Deadline);
    assert!(matches!(
        converted,
        ClientError::Transport(TransportError::Deadline)
    ));
}

#[test]
fn local_credential_validation_accepts_only_the_effective_user() {
    assert!(validate_uid(LOCAL_UID, LOCAL_UID).is_ok());
    assert!(matches!(
        validate_uid(OTHER_UID, LOCAL_UID),
        Err(ClientError::WrongLocalUser)
    ));
}
