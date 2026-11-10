use super::constants::SECRET;
use crate::RelayError;
use std::error::Error;
use yolo_hub_client::ClientError;
use yolo_hub_jsonrpc_transport::TransportError;

#[test]
fn terminal_errors_do_not_expose_remote_payloads_or_sensitive_error_sources() {
    let transport = || TransportError::Io(std::io::Error::other(SECRET));
    for error in [
        RelayError::from(std::io::Error::other(SECRET)),
        RelayError::StatusClosed,
        RelayError::OutputDeadline,
        RelayError::Unreachable,
        RelayError::ResyncRequired,
        RelayError::InvalidReply,
        RelayError::InvalidOptions,
        RelayError::from(ClientError::Transport(transport())),
        RelayError::from(ClientError::OutcomeUnknown(transport())),
    ] {
        assert!(!error.to_string().is_empty());
        assert!(!format!("{error} {error:?} {error:#}").contains(SECRET));
        assert!(error.source().is_none());
    }
}
