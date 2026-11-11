use crate::protocol::{ErrorBody, ProtocolError};
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("{code}: {message}")]
    Remote {
        code: String,
        message: String,
        retryable: bool,
        details: Option<serde_json::Value>,
    },
    #[error("invalid managed local home {home:?}: {message}")]
    InvalidLocalHome { home: PathBuf, message: String },
    #[error("host reported invalid local home {home:?}: {message}")]
    InvalidHostHome { home: PathBuf, message: String },
    #[error("host protocol version mismatch: expected {expected}, reported {reported}")]
    HostProtocolMismatch { expected: u16, reported: u16 },
    #[error("host does not support capability {capability}")]
    HostCapabilityUnavailable { capability: String },
    #[error("host home mismatch at {socket:?}: expected {expected:?}, reported {reported:?}")]
    HostHomeMismatch {
        socket: PathBuf,
        expected: PathBuf,
        reported: PathBuf,
    },
    #[error("terminal {terminal_id} has no current runtime")]
    RuntimeUnavailable { terminal_id: Uuid },
    #[error("host did not attest a complete catalog inventory")]
    IncompleteCatalogInventory,
    #[error("client operation deadline exceeded")]
    Deadline,
}

impl From<ErrorBody> for ClientError {
    fn from(error: ErrorBody) -> Self {
        Self::Remote {
            code: error.code,
            message: error.message,
            retryable: error.retryable,
            details: error.details,
        }
    }
}
