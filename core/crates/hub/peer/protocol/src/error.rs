#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("peer message signature or purpose does not match")]
    Signature,
    #[error("peer message could not be encoded")]
    Encoding,
    #[error("peer message or catalogue exceeds its bounds")]
    Bounds,
    #[error("peer message is expired or not yet valid")]
    Expired,
    #[error("peer route does not match its authenticated connection")]
    Route,
    #[error("peer reply does not match its request")]
    Correlation,
    #[error(transparent)]
    Delegation(#[from] yolo_hub_delegation::DelegationError),
    #[error(transparent)]
    Public(#[from] yolo_hub_terminal_protocol::PublicError),
}
