use anyhow::Result;
use std::time::Duration;
use tokio::time::Instant;
use yolo_conversation_protocol::{
    ConversationBinding, ConversationDescriptor, ConversationLimits, ConversationPage,
    ConversationReadRequest, ConversationReadResult,
};
use yolo_crypto::Sha256Digest;
use yolo_hub_client::LocalClient;
use yolo_hub_conversation_client::{self as conversation, ConversationClientError};
use yolo_hub_identity::{AgentTarget, RuntimeTarget};

const RPC_TIMEOUT: Duration = Duration::from_secs(5);

pub struct Baseline {
    pub descriptor: Option<ConversationDescriptor>,
    pub generation: Option<Sha256Digest>,
    pub revision: Option<Sha256Digest>,
    pub ids: Vec<Sha256Digest>,
    pub native_ids: Vec<String>,
}

impl Baseline {
    pub fn unsupported() -> Self {
        Self {
            descriptor: None,
            generation: None,
            revision: None,
            ids: Vec::new(),
            native_ids: Vec::new(),
        }
    }

    pub fn page(descriptor: ConversationDescriptor, page: ConversationPage) -> Self {
        Self {
            descriptor: Some(descriptor),
            generation: Some(page.history_generation),
            revision: Some(page.revision),
            ids: page.entries.iter().map(|entry| entry.id).collect(),
            native_ids: page
                .entries
                .iter()
                .filter_map(|entry| entry.native_id.clone())
                .collect(),
        }
    }
}

pub fn latest_request(
    descriptor: &ConversationDescriptor,
    revision: Option<Sha256Digest>,
    generation: Option<Sha256Digest>,
) -> ConversationReadRequest {
    ConversationReadRequest {
        address: descriptor.address.clone(),
        binding: descriptor.binding,
        cursor: None,
        limits: ConversationLimits::default(),
        since_revision: revision,
        since_generation: generation,
    }
}

pub async fn capture(
    client: &mut LocalClient,
    target: AgentTarget,
    runtime: RuntimeTarget,
) -> Result<Baseline> {
    Ok(current(client, target, runtime)
        .await?
        .map(|(descriptor, page)| Baseline::page(descriptor, page))
        .unwrap_or_else(Baseline::unsupported))
}

/// Resolve the native conversation and read its latest page. `None` means the
/// provider has no usable conversation for this execution.
pub async fn current(
    client: &mut LocalClient,
    target: AgentTarget,
    runtime: RuntimeTarget,
) -> Result<Option<(ConversationDescriptor, ConversationPage)>> {
    let deadline = Instant::now() + RPC_TIMEOUT;
    let descriptor = match conversation::resolve(
        client,
        ConversationBinding {
            agent: target,
            runtime,
        },
        deadline,
    )
    .await
    {
        Ok(descriptor) => descriptor,
        Err(ConversationClientError::Conversation(_)) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let request = latest_request(&descriptor, None, None);
    match conversation::read(client, request, deadline).await {
        Ok(ConversationReadResult::Page { page }) => Ok(Some((descriptor, page))),
        Ok(ConversationReadResult::Unchanged { .. }) => Ok(None),
        Err(ConversationClientError::Conversation(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}
