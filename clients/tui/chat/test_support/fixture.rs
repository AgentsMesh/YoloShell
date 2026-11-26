#[path = "constants.rs"]
pub mod constants;
#[path = "snapshot.rs"]
mod snapshot;
pub use constants::*;
pub use snapshot::{agent, binding};
use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;
use yolo_hub_agent_protocol::{AgentControlRequest, CommandOutcome, CommandReceipt};
use yolo_tui_chat::{Chat, Completion, Mode, ReadKind};

pub fn descriptor(binding: ConversationBinding) -> ConversationDescriptor {
    ConversationDescriptor {
        address: ConversationAddress {
            hub_id: binding.agent.address.hub_id,
            conversation_id: Sha256Digest::capture(&NATIVE_SESSION).unwrap(),
        },
        binding,
        provider: ConversationProvider::codex(),
        provider_session_id: NATIVE_SESSION.into(),
        source: ConversationSource::NativeTranscript,
    }
}
pub fn ready() -> (Chat, yolo_hub_agent_protocol::AgentSnapshot) {
    let agent = agent();
    let mut chat = Chat::new(binding(&agent)).unwrap();
    chat.set_mode(Mode::Chat);
    let query = chat.resolve().unwrap();
    assert_eq!(
        chat.resolved(&query, descriptor(binding(&agent))),
        Completion::Applied
    );
    (chat, agent)
}
pub fn entry(seed: u8) -> ConversationEntry {
    ConversationEntry {
        id: Sha256Digest::repeated(seed),
        parent_id: None,
        native_id: None,
        timestamp: None,
        role: ConversationRole::Assistant,
        parts: vec![ConversationPart::Text {
            text: format!("message {seed}"),
        }],
    }
}
pub fn page(chat: &Chat, ids: &[u8]) -> ConversationPage {
    let descriptor = chat.descriptor().unwrap().clone();
    let generation = Sha256Digest::repeated(HISTORY_SEED);
    ConversationPage {
        conversation: descriptor.clone(),
        history_generation: generation,
        revision: Sha256Digest::capture(&ids).unwrap(),
        availability: ConversationAvailability::Available,
        partial: vec![],
        entries: ids.iter().copied().map(entry).collect(),
        next_cursor: Some(ConversationCursor {
            address: descriptor.address,
            history_generation: generation,
            before_offset: START_OFFSET,
        }),
    }
}
pub fn load(chat: &mut Chat, page: ConversationPage) {
    let query = chat.read(ReadKind::Latest).unwrap();
    assert_eq!(
        chat.loaded(&query, ConversationReadResult::Page { page }),
        Completion::Applied
    );
}
pub fn receipt(request: &AgentControlRequest, outcome: CommandOutcome) -> CommandReceipt {
    let mut receipt =
        CommandReceipt::accepted(request.fields().request_id, request.fields().target).unwrap();
    receipt.advance(outcome).unwrap();
    receipt
}
