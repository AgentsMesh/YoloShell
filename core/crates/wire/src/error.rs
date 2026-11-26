use super::ErrorBody;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("protocol I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol encode error: {0}")]
    Encode(yolo_capnp_value::ValueError),
    #[error("protocol decode error: {0}")]
    Decode(yolo_capnp_value::ValueError),
    #[error("protocol frame length {length} exceeds limit")]
    FrameTooLarge { length: usize },
    #[error("protocol stream ended after {received} frame-header bytes")]
    TruncatedHeader { received: usize },
    #[error("protocol stream ended")]
    EndOfStream,
    #[error("response envelope does not match its request")]
    ResponseMismatch,
    #[error("event envelope does not match its attachment")]
    EventMismatch,
    #[error("attachment result does not match its selected runtime")]
    AttachmentMismatch,
    #[error("protocol connection is poisoned")]
    ConnectionPoisoned,
    #[error("operation outcome is unknown; do not replay the request")]
    OutcomeUnknown,
    #[error("remote protocol error: {0:?}")]
    Remote(ErrorBody),
}
