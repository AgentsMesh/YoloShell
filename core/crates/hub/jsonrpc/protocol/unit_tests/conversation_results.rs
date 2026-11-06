use serde_json::json;
use yolo_conversation_protocol::{
    ConversationDescriptor, ConversationProvider, ConversationReadResult, ConversationSource,
    MAX_WATCH_WAIT_MS,
};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_rpc_protocol_test_support::commands;
use yolo_hub_terminal_protocol::PublicError;

const NATIVE_SESSION: &str = "native-conversation";
const READING: &str = "reading";
const REVISION: &str = "revision";
const FOREIGN_DIGEST: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn resolved_conversation_cannot_rebind_the_selected_agent_execution_to_a_foreign_terminal() {
    let request = commands::resources::conversation_read();
    let conversation = ConversationDescriptor {
        address: request.address,
        binding: request.binding,
        provider: ConversationProvider::codex(),
        provider_session_id: NATIVE_SESSION.into(),
        source: ConversationSource::NativeTranscript,
    };
    let call = Call::from_request(commands::request(Command::ResolveConversation {
        binding: conversation.binding,
    }));
    let expected = Reply::Conversation {
        conversation: conversation.clone(),
    };
    let answer = Answer::new(&call, expected.clone()).unwrap();
    let bytes = serde_json::to_vec(&answer).unwrap();
    assert_eq!(
        serde_json::from_slice::<Answer>(&bytes)
            .unwrap()
            .reply(&call)
            .unwrap(),
        expected
    );
    let mut foreign = conversation;
    foreign.binding.runtime.runtime_epoch = RuntimeEpoch::new();
    assert_eq!(
        Answer::new(
            &call,
            Reply::Conversation {
                conversation: foreign
            }
        )
        .unwrap()
        .reply(&call),
        Err(PublicError::InvalidPayload)
    );
}

#[test]
fn unchanged_read_and_watch_replies_are_bound_to_both_requested_history_tokens() {
    let mut request = commands::resources::conversation_read();
    let digest = request.address.conversation_id;
    request.since_revision = Some(digest);
    request.since_generation = Some(digest);
    for command in [
        Command::ReadConversation {
            request: request.clone(),
        },
        Command::WatchConversation {
            request: request.clone(),
            wait_ms: MAX_WATCH_WAIT_MS,
        },
    ] {
        let call = Call::from_request(commands::request(command));
        let reply = Reply::ConversationRead {
            reading: ConversationReadResult::Unchanged {
                history_generation: digest,
                revision: digest,
            },
        };
        let mut answer = Answer::new(&call, reply.clone()).unwrap();
        assert_eq!(answer.clone().reply(&call).unwrap(), reply);
        answer.result.as_mut().unwrap()[READING][REVISION] = json!(FOREIGN_DIGEST);
        assert_eq!(answer.reply(&call), Err(PublicError::InvalidPayload));
    }
}
