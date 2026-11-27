use crate::constants::*;
use yolo_hub_client_access_protocol::AccessError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RegistryError {
    #[error(transparent)]
    Access(#[from] AccessError),
    #[error("{CLOSED}")]
    Closed,
    #[error("{REVISION_CONFLICT}")]
    RevisionConflict,
    #[error("{WRONG_ENTRY}")]
    WrongEntry,
    #[error("{REVISION_EXHAUSTED}")]
    RevisionExhausted,
    #[error("{UNAVAILABLE}")]
    Unavailable,
    #[error("{UNAUTHORIZED}")]
    Unauthorized,
}
