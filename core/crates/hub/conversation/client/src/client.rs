use crate::ConversationClientError;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use yolo_conversation_protocol::*;
use yolo_hub_client::Client;
use yolo_hub_identity::{AgentAddress, AgentTarget};
use yolo_hub_rpc_protocol::{Command, Reply};

/// Chooses the current execution once; later server checks reject a replaced execution.
pub async fn resolve_current<S: AsyncRead + AsyncWrite + Unpin>(
    client: &mut Client<S>,
    address: AgentAddress,
    deadline: Instant,
) -> Result<ConversationDescriptor, ConversationClientError> {
    let Reply::Agent { agent } = client
        .call(address.hub_id, Command::GetCurrentAgent { address }, deadline)
        .await?
    else {
        return Err(ConversationClientError::UnexpectedReply);
    };
    resolve(
        client,
        ConversationBinding {
            agent: agent.cursor().target(),
            runtime: agent.fields().terminal,
        },
        deadline,
    )
    .await
}

/// Revalidates the requested agent execution and discovers its terminal association.
pub async fn resolve_agent<S: AsyncRead + AsyncWrite + Unpin>(
    client: &mut Client<S>,
    target: AgentTarget,
    deadline: Instant,
) -> Result<ConversationDescriptor, ConversationClientError> {
    let reply = client
        .call(
            target.address.hub_id,
            Command::GetAgent { target },
            deadline,
        )
        .await?;
    let Reply::Agent { agent } = reply else {
        return Err(ConversationClientError::UnexpectedReply);
    };
    if agent.cursor().target() != target {
        return Err(ConversationError::BindingChanged.into());
    }
    resolve(
        client,
        ConversationBinding {
            agent: target,
            runtime: agent.fields().terminal,
        },
        deadline,
    )
    .await
}

pub async fn resolve<S: AsyncRead + AsyncWrite + Unpin>(
    client: &mut Client<S>,
    binding: ConversationBinding,
    deadline: Instant,
) -> Result<ConversationDescriptor, ConversationClientError> {
    binding.validate()?;
    match client
        .call(
            binding.agent.address.hub_id,
            Command::ResolveConversation { binding },
            deadline,
        )
        .await?
    {
        Reply::Conversation { conversation } => {
            conversation.validate_for(binding)?;
            Ok(conversation)
        }
        Reply::ConversationUnavailable { error } => Err(error.into()),
        _ => Err(ConversationClientError::UnexpectedReply),
    }
}

pub async fn read<S: AsyncRead + AsyncWrite + Unpin>(
    client: &mut Client<S>,
    request: ConversationReadRequest,
    deadline: Instant,
) -> Result<ConversationReadResult, ConversationClientError> {
    request.validate()?;
    let reply = client
        .call(
            request.address.hub_id,
            Command::ReadConversation {
                request: request.clone(),
            },
            deadline,
        )
        .await?;
    reading(reply, &request)
}

/// One bounded conditional watch; callers decide whether and when to resubscribe.
pub async fn watch<S: AsyncRead + AsyncWrite + Unpin>(
    client: &mut Client<S>,
    request: ConversationReadRequest,
    wait_ms: u32,
    deadline: Instant,
) -> Result<ConversationReadResult, ConversationClientError> {
    request.validate()?;
    if wait_ms > MAX_WATCH_WAIT_MS {
        return Err(ConversationError::InvalidRequest.into());
    }
    let reply = client
        .call(
            request.address.hub_id,
            Command::WatchConversation {
                request: request.clone(),
                wait_ms,
            },
            deadline,
        )
        .await?;
    reading(reply, &request)
}

fn reading(
    reply: Reply,
    request: &ConversationReadRequest,
) -> Result<ConversationReadResult, ConversationClientError> {
    match reply {
        Reply::ConversationRead { reading } => {
            reading.validate_for(request)?;
            Ok(reading)
        }
        Reply::ConversationUnavailable { error } => Err(error.into()),
        _ => Err(ConversationClientError::UnexpectedReply),
    }
}
