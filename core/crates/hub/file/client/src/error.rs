#[derive(Debug, thiserror::Error)]
pub enum FileClientError {
    #[error(transparent)]
    Client(#[from] yolo_hub_client::ClientError),
    #[error("file roots changed while listing")]
    Changed,
    #[error("file roots exceed the origin limit")]
    Limit,
    #[error("unexpected file response")]
    UnexpectedReply,
}
