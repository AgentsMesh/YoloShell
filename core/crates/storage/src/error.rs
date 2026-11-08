use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("storage path must be absolute and normalized")]
    InvalidPath,
    #[error("storage value exceeds the configured byte limit")]
    Limit,
    #[error("storage value is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("storage I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("session identifier must not be nil")]
    NilSession,
}
