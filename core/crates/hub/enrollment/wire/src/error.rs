use crate::JoinErrorCode;

#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error("enrollment frame I/O failed: {0}")]
    Io(std::io::Error),
    #[error("enrollment frame is not valid JSON")]
    Codec,
    #[error("enrollment frame exceeds its size bound")]
    Size,
    #[error("enrollment peer rejected the request: {0}")]
    Rejected(JoinErrorCode),
}
