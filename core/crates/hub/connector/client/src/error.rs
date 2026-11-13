use crate::constants::*;
use std::{error::Error, fmt};
use yolo_hub_connector_protocol::ConnectorError;
use yolo_hub_jsonrpc_transport::TransportError;

pub enum ClientError {
    Validation(ConnectorError),
    Remote(ConnectorError),
    InvalidResponse(ConnectorError),
    Transport(TransportError),
    OutcomeUnknown(TransportError),
    OutcomeUnknownResponse(ConnectorError),
    Poisoned,
    WrongLocalUser,
}

impl ClientError {
    pub fn outcome_is_unknown(&self) -> bool {
        matches!(
            self,
            Self::OutcomeUnknown(_)
                | Self::OutcomeUnknownResponse(_)
                | Self::Remote(ConnectorError::OutcomeUnknown)
        )
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Validation(_) => INVALID_REQUEST,
            Self::InvalidResponse(_) => INVALID_RESPONSE,
            Self::Remote(ConnectorError::OutcomeUnknown)
            | Self::OutcomeUnknown(_)
            | Self::OutcomeUnknownResponse(_) => UNKNOWN_OUTCOME,
            Self::Remote(_) => REMOTE_REJECTION,
            Self::Transport(_) => TRANSPORT_FAILED,
            Self::Poisoned => RETIRED,
            Self::WrongLocalUser => WRONG_USER,
        };
        formatter.write_str(label)
    }
}

impl fmt::Debug for ClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

// Wire decoders may include credential bytes in errors. Keep diagnostics and chains redacted.
impl Error for ClientError {}

impl From<TransportError> for ClientError {
    fn from(error: TransportError) -> Self {
        Self::Transport(error)
    }
}
