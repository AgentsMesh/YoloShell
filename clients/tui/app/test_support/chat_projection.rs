use crate::{
    State,
    chat::{
        self, Key,
        network::{Operation, Reading},
    },
    network::Job,
};
use tokio::sync::mpsc;
use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;

pub fn operation(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    incoming: &mut mpsc::Receiver<Job>,
) -> Operation {
    let key = Key::current(state);
    if let Some(entry) = state.chats.entries.get_mut(&key) {
        entry.next = std::time::Instant::now();
    }
    chat::pump(state, jobs);
    let Job::Chat(operation) = incoming.try_recv().unwrap() else {
        panic!()
    };
    operation
}
pub fn descriptor(state: &State) -> ConversationDescriptor {
    let binding = chat::current(state).unwrap().binding();
    ConversationDescriptor {
        address: ConversationAddress {
            hub_id: binding.agent.address.hub_id,
            conversation_id: Sha256Digest::capture(&super::chat_support::NATIVE_SESSION).unwrap(),
        },
        binding,
        provider: ConversationProvider::codex(),
        provider_session_id: super::chat_support::NATIVE_SESSION.into(),
        source: ConversationSource::NativeTranscript,
    }
}
pub fn resolve(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    incoming: &mut mpsc::Receiver<Job>,
) -> ConversationDescriptor {
    let operation = operation(state, jobs, incoming);
    let descriptor = descriptor(state);
    chat::completed(state, operation, Ok(Reading::Resolved(descriptor.clone())));
    descriptor
}
pub fn page(descriptor: &ConversationDescriptor, seed: u8, offset: u64) -> ConversationPage {
    ConversationPage {
        conversation: descriptor.clone(),
        history_generation: Sha256Digest::capture(&descriptor.address).unwrap(),
        revision: Sha256Digest::capture(&seed).unwrap(),
        availability: ConversationAvailability::Available,
        partial: vec![],
        entries: vec![ConversationEntry {
            id: Sha256Digest::capture(&seed).unwrap(),
            parent_id: None,
            native_id: None,
            timestamp: None,
            role: ConversationRole::Assistant,
            parts: vec![ConversationPart::Text {
                text: format!("reply {seed}"),
            }],
        }],
        next_cursor: Some(ConversationCursor {
            address: descriptor.address.clone(),
            history_generation: Sha256Digest::capture(&descriptor.address).unwrap(),
            before_offset: offset,
        }),
    }
}
