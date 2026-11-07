use std::fmt;
use yolo_presentation_protocol::ProtocolError;

#[derive(Debug)]
pub enum StoreError {
    Protocol(ProtocolError),
    Storage(anyhow::Error),
}
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => error.fmt(f),
            Self::Storage(error) => write!(f, "{error:#}"),
        }
    }
}
impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Protocol(error) => error,
            Self::Storage(error) => error.as_ref(),
        })
    }
}
impl From<ProtocolError> for StoreError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}
impl From<anyhow::Error> for StoreError {
    fn from(value: anyhow::Error) -> Self {
        Self::Storage(value)
    }
}
