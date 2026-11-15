use yolo_hub_jsonrpc_transport::TransportError;
use yolo_hub_terminal_protocol::PublicError;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Hub request validation failed: {0}")]
    Validation(PublicError),
    #[error("Hub request was rejected: {0}")]
    Remote(PublicError),
    #[error("Hub response does not match its request: {0}")]
    InvalidResponse(PublicError),
    #[error("Hub transport failed: {0}")]
    Transport(#[from] TransportError),
    #[error("Hub operation outcome is unknown; do not replay automatically: {0}")]
    OutcomeUnknown(#[source] TransportError),
    #[error("Hub operation outcome is unknown because its response was invalid: {0}")]
    OutcomeUnknownResponse(PublicError),
    #[error("Hub connection is no longer usable")]
    Poisoned,
    #[error("Hub connection is a terminal event stream, not a control channel")]
    TerminalStream,
    #[error("Hub does not support this operation")]
    Unsupported,
    #[error("Direct SSH Hub entry requires a pinned remote profile and client credential")]
    UnsupportedEndpoint,
    #[error("Hub endpoint is invalid")]
    InvalidEndpoint,
    #[error("local Hub peer is not owned by the current user")]
    WrongLocalUser,
}

impl ClientError {
    pub fn outcome_is_unknown(&self) -> bool {
        matches!(
            self,
            Self::OutcomeUnknown(_)
                | Self::OutcomeUnknownResponse(_)
                | Self::Remote(PublicError::OutcomeUnknown)
        )
    }
}

impl From<yolo_hub_terminal_wire::Error> for ClientError {
    fn from(error: yolo_hub_terminal_wire::Error) -> Self {
        match error {
            yolo_hub_terminal_wire::Error::Transport(error) => Self::Transport(error),
            yolo_hub_terminal_wire::Error::Validation(error) => Self::InvalidResponse(error),
        }
    }
}
