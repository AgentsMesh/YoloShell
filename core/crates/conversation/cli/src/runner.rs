use crate::arguments::{Action, Options};
use crate::constants::{NO_LOCAL_HUB, SOCKET_PAIR};
use anyhow::Context;
use std::time::Duration;
use tokio::time::Instant;
use yolo_conversation_protocol::{ConversationError, ConversationLimits, ConversationReadRequest};
use yolo_hub_client::LocalClient;
use yolo_hub_conversation_client::{self as conversation, ConversationClientError};
use yolo_hub_identity::{AgentAddress, HubId};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_state::HubState;
use yolo_local_profile::LocalProfile;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) async fn run(
    options: Options,
    request: Option<ConversationReadRequest>,
) -> anyhow::Result<Reply> {
    let deadline = Instant::now() + REQUEST_TIMEOUT;
    let (mut client, hub) = connect(&options, deadline).await?;
    let result: Result<Reply, ConversationClientError> = async {
        match options.action {
            Action::Resolve { agent, origin } => {
                let address = AgentAddress {
                    hub_id: origin.unwrap_or(hub),
                    agent_id: agent,
                };
                Ok(Reply::Conversation {
                    conversation: conversation::resolve_current(&mut client, address, deadline)
                        .await?,
                })
            }
            Action::Read {
                agent: Some(agent),
                origin,
            } => {
                let address = AgentAddress {
                    hub_id: origin.unwrap_or(hub),
                    agent_id: agent,
                };
                let descriptor =
                    conversation::resolve_current(&mut client, address, deadline).await?;
                let request = ConversationReadRequest {
                    address: descriptor.address,
                    binding: descriptor.binding,
                    cursor: None,
                    limits: ConversationLimits::default(),
                    since_revision: None,
                    since_generation: None,
                };
                Ok(Reply::ConversationRead {
                    reading: conversation::read(&mut client, request, deadline).await?,
                })
            }
            Action::Read { agent: None, .. } => {
                let request = request.ok_or(ConversationError::InvalidRequest)?;
                Ok(Reply::ConversationRead {
                    reading: conversation::read(&mut client, request, deadline).await?,
                })
            }
            Action::Watch { wait_ms } => {
                let request = request.ok_or(ConversationError::InvalidRequest)?;
                Ok(Reply::ConversationRead {
                    reading: conversation::watch(&mut client, request, wait_ms, deadline).await?,
                })
            }
        }
    }
    .await;
    match result {
        Ok(reply) => Ok(reply),
        Err(ConversationClientError::Conversation(error)) => {
            Ok(Reply::ConversationUnavailable { error })
        }
        Err(error) => Err(error).context("conversation request failed"),
    }
}

async fn connect(
    options: &Options,
    deadline: Instant,
) -> anyhow::Result<(LocalClient, HubId)> {
    match (options.socket.as_ref(), options.hub) {
        (Some(socket), Some(hub)) => {
            Ok((LocalClient::connect_local(socket, hub, deadline).await?, hub))
        }
        (None, None) => {
            let profile = match &options.local_profile {
                Some(path) => LocalProfile::load(path)?,
                None => yolo_local_profile::current()?,
            };
            let hub = HubState::inspect_identity(&profile.state, &profile.host_home)?
                .context(NO_LOCAL_HUB)?;
            let socket = HubState::socket_path_for(&profile.state)?;
            Ok((LocalClient::connect_local(&socket, hub, deadline).await?, hub))
        }
        _ => anyhow::bail!(SOCKET_PAIR),
    }
}
