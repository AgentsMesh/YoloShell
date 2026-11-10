use super::{super::federation_support::*, constants::*};
use std::io::Write;
use yolo_conversation_protocol::*;
use yolo_hub_conversation_client::{self as conversation, ConversationClientError};
use yolo_hub_federation_settings::ExportPolicy;
use yolo_hub_identity::{AgentAddress, HubId};
use yolo_hub_routing::Right;

pub fn export(origin: HubId, relay: bool) -> ExportPolicy {
    let mut policy = exports(origin, relay);
    policy.rights = policy
        .rights
        .union(&[Right::ConversationRead].into_iter().collect());
    policy
}

pub fn append_fixture_content(harness: &crate::Harness, view: &yolo_runtime::model::TerminalView) {
    let path = std::path::Path::new(&view.agent.as_ref().unwrap().evidence_path);
    assert!(path.starts_with(harness.home.path().canonicalize().unwrap()));
    let record = serde_json::json!({"type":RESPONSE_ITEM,"payload":{"type":"message","role":"assistant",
        "content":[{"type":OUTPUT_TEXT,"text":CONTENT_MARKER}]}});
    std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(format!("{record}\n").as_bytes())
        .unwrap();
}

pub fn request(descriptor: &ConversationDescriptor) -> ConversationReadRequest {
    ConversationReadRequest {
        address: descriptor.address.clone(),
        binding: descriptor.binding,
        cursor: None,
        limits: ConversationLimits::default(),
        since_revision: None,
        since_generation: None,
    }
}

pub async fn resolve(entry: &RunningNode, address: AgentAddress) -> ConversationDescriptor {
    tokio::time::timeout_at(deadline(), async {
        loop {
            match conversation::resolve_current(&mut entry.client().await, address, deadline())
                .await
            {
                Ok(descriptor) => return descriptor,
                Err(ConversationClientError::Conversation(_)) => {}
                Err(ConversationClientError::Client(_)) => {}
                Err(error) => panic!("invalid conversation reply: {error}"),
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}
