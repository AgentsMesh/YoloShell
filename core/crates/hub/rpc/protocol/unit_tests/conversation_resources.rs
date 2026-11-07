use crate::commands::{request, resources::conversation_read};
use crate::fixtures::{constants::HUB_ID, hub};
use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;
use yolo_hub_rpc_protocol::{Command, Reply, Response};
use yolo_hub_terminal_protocol::PublicError;

const GENERATION_SEED: &str = "history-generation";
const REVISION_SEED: &str = "content-revision";
const NATIVE_SESSION: &str = "native-conversation";

#[test]
fn conversation_pages_bind_the_execution_and_conditional_history_checkpoint() {
    let mut reading = conversation_read();
    let generation = Sha256Digest::capture(&GENERATION_SEED).unwrap();
    let revision = Sha256Digest::capture(&REVISION_SEED).unwrap();
    let descriptor = ConversationDescriptor {
        address: reading.address.clone(),
        binding: reading.binding,
        provider: ConversationProvider::codex(),
        provider_session_id: NATIVE_SESSION.into(),
        source: ConversationSource::NativeTranscript,
    };
    let resolve = request(Command::ResolveConversation {
        binding: reading.binding,
    });
    assert!(
        Response::new(
            &resolve,
            hub(HUB_ID),
            Reply::Conversation {
                conversation: descriptor.clone()
            }
        )
        .validate_for(&resolve)
        .is_ok()
    );
    let page = ConversationPage {
        conversation: descriptor,
        history_generation: generation,
        revision,
        availability: ConversationAvailability::Available,
        partial: vec![],
        entries: vec![],
        next_cursor: None,
    };
    let read = request(Command::ReadConversation {
        request: reading.clone(),
    });
    assert!(
        Response::new(
            &read,
            hub(HUB_ID),
            Reply::ConversationRead {
                reading: ConversationReadResult::Page { page: page.clone() }
            }
        )
        .validate_for(&read)
        .is_ok()
    );
    let unchanged = Reply::ConversationRead {
        reading: ConversationReadResult::Unchanged {
            history_generation: generation,
            revision,
        },
    };
    assert_eq!(
        Response::new(&read, hub(HUB_ID), unchanged.clone()).validate_for(&read),
        Err(PublicError::InvalidPayload)
    );
    reading.since_revision = Some(revision);
    reading.since_generation = Some(generation);
    let watch = request(Command::WatchConversation {
        request: reading,
        wait_ms: MAX_WATCH_WAIT_MS,
    });
    assert!(
        Response::new(&watch, hub(HUB_ID), unchanged)
            .validate_for(&watch)
            .is_ok()
    );
    let mut wrong = page;
    wrong.conversation.binding.agent.execution_id = yolo_hub_identity::ExecutionId::new();
    assert_eq!(
        Response::new(
            &read,
            hub(HUB_ID),
            Reply::ConversationRead {
                reading: ConversationReadResult::Page { page: wrong }
            }
        )
        .validate_for(&read),
        Err(PublicError::InvalidPayload)
    );
}
