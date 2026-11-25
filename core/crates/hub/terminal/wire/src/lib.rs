mod attachment;
mod event;
mod metadata;

pub use attachment::{receive_attachment, send_attachment};
pub use event::{receive_event, send_event};
pub const STREAM_ENCODING: &str = "yolo.terminal.binary.v1";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("terminal stream transport failed: {0}")]
    Transport(#[from] yolo_hub_jsonrpc_transport::TransportError),
    #[error("terminal stream metadata is invalid: {0}")]
    Validation(#[from] yolo_hub_terminal_protocol::PublicError),
}

impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Self::Validation(yolo_hub_terminal_protocol::PublicError::InvalidPayload)
    }
}
