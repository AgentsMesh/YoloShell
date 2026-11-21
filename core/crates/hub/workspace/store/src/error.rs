use yolo_hub_terminal_protocol::PublicError;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Protocol(#[from] PublicError),
    #[error(transparent)]
    Storage(#[from] anyhow::Error),
}

impl StoreError {
    pub fn into_public(self) -> PublicError {
        match self {
            Self::Protocol(error) => error,
            Self::Storage(error) => {
                tracing::error!(%error, "workspace store failure");
                PublicError::Internal
            }
        }
    }
}
