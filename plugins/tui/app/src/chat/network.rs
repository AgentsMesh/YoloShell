use super::{Key, constants::*};
use crate::{Options, constants::RPC_TIMEOUT, network::Update};
use tokio::{sync::mpsc, time::Instant};
use uuid::Uuid;
use yolo_conversation_protocol::{
    ConversationDescriptor, ConversationReadResult, MAX_WATCH_WAIT_MS,
};
use yolo_hub_client::ClientError;
use yolo_hub_conversation_client::ConversationClientError;
use yolo_hub_terminal_protocol::PublicError;
use yolo_tui_chat::{Query, QueryError, QueryKind, ReadKind};

pub(crate) const CAPACITY: usize = QUERY_CAPACITY;
#[derive(Clone, Debug)]
pub(crate) struct Operation {
    pub key: Key,
    pub generation: Uuid,
    pub query: Query,
}
pub(crate) enum Reading {
    Resolved(ConversationDescriptor),
    Page(ConversationReadResult),
}

pub(crate) async fn run(
    options: &Options,
    mut incoming: mpsc::Receiver<Operation>,
    updates: &mpsc::Sender<Update>,
) {
    while let Some(operation) = incoming.recv().await {
        let result = execute(options, &operation).await;
        if updates
            .send(Update::Chat { operation, result })
            .await
            .is_err()
        {
            break;
        }
    }
}
async fn execute(options: &Options, operation: &Operation) -> Result<Reading, QueryError> {
    let deadline = Instant::now() + RPC_TIMEOUT;
    let mut client = options
        .endpoint
        .connect(options.hub, deadline)
        .await
        .map_err(|_| QueryError::Disconnected)?;
    match operation.query.kind() {
        QueryKind::Resolve { binding } => {
            yolo_hub_conversation_client::resolve(&mut client, *binding, deadline)
                .await
                .map(Reading::Resolved)
        }
        QueryKind::Read { request, kind }
            if *kind == ReadKind::Latest && request.since_revision.is_some() =>
        {
            yolo_hub_conversation_client::watch(
                &mut client,
                request.clone(),
                MAX_WATCH_WAIT_MS,
                deadline,
            )
            .await
            .map(Reading::Page)
        }
        QueryKind::Read { request, .. } => {
            yolo_hub_conversation_client::read(&mut client, request.clone(), deadline)
                .await
                .map(Reading::Page)
        }
    }
    .map_err(|error| match error {
        ConversationClientError::Conversation(error) => QueryError::Conversation(error),
        ConversationClientError::Client(
            ClientError::Unsupported | ClientError::Remote(PublicError::Unsupported),
        ) => QueryError::Unsupported,
        ConversationClientError::Client(_) => QueryError::Disconnected,
        ConversationClientError::UnexpectedReply => QueryError::InvalidResponse,
    })
}
