#[derive(Debug, thiserror::Error)]
pub enum ConversationClientError {
    #[error(transparent)]
    Client(#[from] yolo_hub_client::ClientError),
    #[error(transparent)]
    Conversation(#[from] yolo_conversation_protocol::ConversationError),
    #[error("unexpected conversation response")]
    UnexpectedReply,
}
