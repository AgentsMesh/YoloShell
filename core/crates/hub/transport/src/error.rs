#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("public Hub frame is empty")]
    EmptyFrame,
    #[error("public Hub frame exceeds its payload limit")]
    FrameTooLarge,
    #[error("public Hub frame contains trailing bytes")]
    TrailingBytes,
    #[error("public Hub stream ended inside a frame")]
    TruncatedFrame,
    #[error("public Hub frame encoding failed: {0}")]
    Encode(#[source] yolo_capnp_codec::ValueError),
    #[error("public Hub frame decoding failed: {0}")]
    Decode(#[source] yolo_capnp_codec::ValueError),
    #[error("public Hub transport failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Hub did not respond in time; retry shortly")]
    Deadline,
    #[error("public Hub connection is no longer reusable")]
    Poisoned,
}
