use serde_json::{Value, json};
use yolo_conversation_protocol::*;
use yolo_hub_jsonrpc_protocol::{CHAT_METHOD, Call, ClientCall, INVALID_PARAMS};
use yolo_hub_rpc_protocol::{Command, Method, Reply};
use yolo_hub_rpc_protocol_test_support::{commands, fixtures};

fn wire(command: Command, operation: &str) -> Value {
    let mut value = serde_json::to_value(Call::from_request(commands::request(command))).unwrap();
    value["method"] = json!(CHAT_METHOD);
    value["params"]["operation"] = json!(operation);
    value
}

#[test]
fn profile_validates_binding_and_returns_only_modeled_session_updates() {
    let request = commands::resources::conversation_read();
    let digest = request.address.conversation_id;
    let conversation = ConversationDescriptor {
        address: request.address.clone(),
        binding: request.binding,
        provider: ConversationProvider::codex(),
        provider_session_id: "private-native-id".into(),
        source: ConversationSource::NativeTranscript,
    };
    let open = ClientCall::parse(wire(
        Command::ResolveConversation {
            binding: request.binding,
        },
        "open",
    ))
    .unwrap();
    let result = open
        .answer(Reply::Conversation {
            conversation: conversation.clone(),
        })
        .unwrap()
        .result
        .unwrap();
    assert_eq!(
        result["sessionId"],
        json!(request.address.chat_session_id())
    );
    assert_eq!(result["version"], 1);
    assert!(!result.to_string().contains("private-native-id"));
    let read = ClientCall::parse(wire(
        Command::ReadConversation {
            request: request.clone(),
        },
        "read",
    ))
    .unwrap();
    let page = ConversationPage {
        conversation,
        history_generation: digest,
        revision: digest,
        availability: ConversationAvailability::Available,
        partial: vec![],
        next_cursor: None,
        entries: vec![ConversationEntry {
            id: digest,
            native_id: Some("private-record".into()),
            parent_id: None,
            timestamp: None,
            role: ConversationRole::Assistant,
            parts: vec![
                ConversationPart::Text {
                    text: "answer".into(),
                },
                ConversationPart::Thinking {
                    text: "thought".into(),
                },
                ConversationPart::ToolCall {
                    call_id: "call".into(),
                    name: "read".into(),
                    arguments: "{}".into(),
                },
                ConversationPart::ToolResult {
                    call_id: "call".into(),
                    output: "value".into(),
                    is_error: false,
                },
                ConversationPart::ImageUnavailable {
                    media_type: Some("image/png".into()),
                },
                ConversationPart::Compaction {
                    summary: "summary".into(),
                },
            ],
        }],
    };
    let mut foreign = page.clone();
    foreign.conversation.binding.agent.execution_id = yolo_hub_identity::ExecutionId::new();
    assert!(
        read.answer(Reply::ConversationRead {
            reading: ConversationReadResult::Page { page: foreign }
        })
        .is_err()
    );
    let result = read
        .answer(Reply::ConversationRead {
            reading: ConversationReadResult::Page { page },
        })
        .unwrap()
        .result
        .unwrap();
    let updates = &result["reading"]["page"]["entries"][0]["updates"];
    assert_eq!(
        updates[0],
        json!({"sessionUpdate":crate::constants::AGENT_MESSAGE_CHUNK,"content":{"type":"text","text":"answer"}})
    );
    assert_eq!(
        updates[1]["sessionUpdate"],
        crate::constants::AGENT_THOUGHT_CHUNK
    );
    assert_eq!(updates[2]["toolCallId"], "call");
    assert_eq!(updates[3]["status"], "completed");
    assert_eq!(updates[3]["content"][0]["content"]["text"], "value");
    assert_eq!(updates[4]["sessionUpdate"], "notice");
    assert_eq!(updates[5]["sessionUpdate"], "compaction_update");
    assert!(!result.to_string().contains("private-record"));
    let mut request = request;
    request.since_revision = Some(digest);
    request.since_generation = Some(digest);
    let wait = ClientCall::parse(wire(
        Command::WatchConversation {
            request,
            wait_ms: 250,
        },
        "read",
    ))
    .unwrap();
    let unchanged = wait
        .answer(Reply::ConversationRead {
            reading: ConversationReadResult::Unchanged {
                revision: digest,
                history_generation: digest,
            },
        })
        .unwrap();
    assert_eq!(unchanged.result.unwrap()["reading"]["status"], "unchanged");
}

#[test]
fn state_and_mutation_receipts_retain_the_original_target_and_request_identity() {
    let target = fixtures::agent().cursor().target();
    let state = ClientCall::parse(wire(Command::GetAgent { target }, "state")).unwrap();
    let agent = fixtures::agent();
    let result = state
        .answer(Reply::Agent {
            agent: agent.clone(),
        })
        .unwrap()
        .result
        .unwrap();
    assert_eq!(result["agent"], json!(agent));

    let command = Command::AgentControl {
        request: fixtures::agent_control(),
        grant: fixtures::grant(),
    };
    let request = commands::request(command.clone());
    let call = ClientCall::parse(wire(command.clone(), "cancel")).unwrap();
    assert_eq!(call.call.fields().request_id, request.fields().request_id);
    assert_eq!(call.call.fields().command, command);
    assert_eq!(call.call.fields().command.method(), Method::AgentControl);
    assert_eq!(
        ClientCall::parse(wire(command.clone(), "respond"))
            .unwrap_err()
            .error
            .code,
        INVALID_PARAMS
    );
    let mut respond = wire(command, "respond");
    respond["params"]["request"]["action"] = json!({
        "operation":"respond", "interaction":{"target":target,"id":uuid::Uuid::new_v4()},
        "response":{"kind":"choice","choice":"yes"}
    });
    let control = ClientCall::parse(respond).unwrap();
    assert_eq!(control.call.fields().command.method(), Method::AgentControl);
}
