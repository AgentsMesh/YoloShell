#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("terminal binary frame does not match its metadata")]
    InvalidBinary,
    #[error("JSON-RPC frame is empty")]
    EmptyFrame,
    #[error("JSON-RPC frame exceeds its payload limit")]
    FrameTooLarge,
    #[error("JSON-RPC stream ended inside a frame")]
    TruncatedFrame,
    #[error("JSON-RPC frame encoding failed: {0}")]
    Encode(#[source] serde_json::Error),
    #[error("JSON-RPC frame decoding failed: {0}")]
    Decode(#[source] serde_json::Error),
    #[error("JSON-RPC transport failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Hub did not respond in time; retry shortly")]
    Deadline,
    #[error("JSON-RPC connection is no longer reusable")]
    Poisoned,
}
