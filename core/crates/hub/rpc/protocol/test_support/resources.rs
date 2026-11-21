use crate::fixtures::{agent, constants::*, hub, id};
use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_presentation_protocol::{LayoutAddress, LayoutSummary, LayoutUpdate};

const CONVERSATION_SEED: &str = "conversation-contract-fixture";

pub fn binding() -> ConversationBinding {
    let agent = agent();
    ConversationBinding {
        agent: agent.cursor().target(),
        runtime: agent.fields().terminal,
    }
}
pub fn conversation_read() -> ConversationReadRequest {
    ConversationReadRequest {
        address: ConversationAddress {
            hub_id: hub(HUB_ID),
            conversation_id: Sha256Digest::capture(&CONVERSATION_SEED).unwrap(),
        },
        binding: binding(),
        cursor: None,
        limits: ConversationLimits::default(),
        since_revision: None,
        since_generation: None,
    }
}
pub fn layout_update() -> LayoutUpdate {
    LayoutUpdate {
        address: LayoutAddress {
            hub_id: hub(HUB_ID),
            layout_id: id(SESSION_ID),
        },
        request_id: id(REQUEST_ID),
        expected_revision: 0,
        tabs: vec![],
    }
}
pub fn samples() -> Vec<(Command, Reply)> {
    let change = layout_update();
    let document = change.apply(None).unwrap();
    vec![
        (
            Command::ListLayouts {},
            Reply::Layouts {
                documents: vec![LayoutSummary::try_from(&document).unwrap()],
            },
        ),
        (
            Command::GetLayout {
                address: change.address,
            },
            Reply::Layout {
                document: Some(document.clone()),
            },
        ),
        (
            Command::UpdateLayout { change },
            Reply::Layout {
                document: Some(document),
            },
        ),
        (
            Command::ResolveConversation { binding: binding() },
            Reply::ConversationUnavailable {
                error: ConversationError::EvidenceUnavailable,
            },
        ),
        (
            Command::ReadConversation {
                request: conversation_read(),
            },
            Reply::ConversationUnavailable {
                error: ConversationError::EvidenceUnavailable,
            },
        ),
        (
            Command::WatchConversation {
                request: conversation_read(),
                wait_ms: MAX_WATCH_WAIT_MS,
            },
            Reply::ConversationUnavailable {
                error: ConversationError::EvidenceUnavailable,
            },
        ),
    ]
}
