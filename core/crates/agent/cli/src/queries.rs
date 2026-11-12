use yolo_agent_catalog as catalog;
use crate::constants::*;
use crate::observe;
use crate::outcome::Outcome;
use crate::render;
use anyhow::Result;
use tokio::time::Instant;
use yolo_conversation_protocol::{
    ConversationBinding, ConversationLimits, ConversationReadRequest,
};
use yolo_hub_client::LocalClient;
use yolo_hub_conversation_client::{self as conversation, ConversationClientError};

pub async fn list(client: &mut LocalClient, all: bool, json: bool) -> Result<Outcome> {
    let rows = catalog::agents(client, all).await?;
    Ok(Outcome::success(
        json,
        render::rows_json(&rows)?,
        render::agents_text(&rows),
    ))
}

pub async fn get(client: &mut LocalClient, selector: String, json: bool) -> Result<Outcome> {
    let selection = catalog::resolve(client, &selector).await?;
    let snapshot = observe::snapshot(client, selection.target).await?;
    let terminal = selection.terminal_name.as_deref();
    Ok(Outcome::success(
        json,
        render::agent_json(&snapshot, terminal)?,
        render::agent_text(&snapshot, terminal),
    ))
}

pub async fn read(
    client: &mut LocalClient,
    selector: String,
    limit: u32,
    json: bool,
) -> Result<Outcome> {
    let selection = catalog::resolve(client, &selector).await?;
    let snapshot = observe::snapshot(client, selection.target).await?;
    let binding = ConversationBinding {
        agent: selection.target,
        runtime: snapshot.fields().terminal,
    };
    let deadline = Instant::now() + RPC_TIMEOUT;
    let descriptor = match conversation::resolve(client, binding, deadline).await {
        Ok(descriptor) => descriptor,
        Err(ConversationClientError::Conversation(error)) => {
            return Ok(unavailable(json, &error));
        }
        Err(error) => return Err(error.into()),
    };
    let request = ConversationReadRequest {
        address: descriptor.address.clone(),
        binding,
        cursor: None,
        limits: ConversationLimits {
            entries: limit,
            ..ConversationLimits::default()
        },
        since_revision: None,
        since_generation: None,
    };
    match conversation::read(client, request, deadline).await {
        Ok(reading) => {
            let value = serde_json::to_value(&reading)?;
            let text = render::conversation_text(&reading);
            Ok(Outcome::success(json, value, text))
        }
        Err(ConversationClientError::Conversation(error)) => Ok(unavailable(json, &error)),
        Err(error) => Err(error.into()),
    }
}

fn unavailable(json: bool, error: &yolo_conversation_protocol::ConversationError) -> Outcome {
    let message = format!("conversation unavailable: {error}");
    Outcome::failure(EXIT_REJECTED, json, message)
}
