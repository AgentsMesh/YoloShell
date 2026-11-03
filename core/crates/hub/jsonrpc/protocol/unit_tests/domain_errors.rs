use serde_json::{Value, json};
use yolo_conversation_protocol::ConversationError;
use yolo_hub_jsonrpc_protocol::{
    Answer, CONVERSATION_ERROR, Call, HUB_ERROR, JSONRPC_VERSION, RpcError,
};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_rpc_protocol_test_support::{commands, fixtures};
use yolo_hub_terminal_protocol::PublicError;

const MESSAGE: &str = "provider rejection";
const UNKNOWN_CODE: i32 = -32099;
use super::constants::KIND;
const CODE: &str = "code";
const ERROR: &str = "error";
const UNKNOWN_FIELD: &str = "untrusted";

#[test]
fn provider_errors_preserve_typed_causes_only_on_conversation_methods() {
    let binding = commands::resources::binding();
    for error in [
        ConversationError::InvalidRequest,
        ConversationError::BindingChanged,
        ConversationError::UnsupportedProvider,
        ConversationError::EvidenceUnavailable,
        ConversationError::EvidenceChanged,
        ConversationError::HistoryChanged,
        ConversationError::HistoryLimitExceeded,
        ConversationError::ReaderBusy,
    ] {
        let call = Call::from_request(commands::request(Command::ResolveConversation { binding }));
        let expected = Reply::ConversationUnavailable { error };
        let answer = Answer::new(&call, expected.clone()).unwrap();
        assert!(answer.result.is_none());
        assert_eq!(answer.error.as_ref().unwrap().code, CONVERSATION_ERROR);
        let wire = serde_json::to_vec(&answer).unwrap();
        let decoded: Answer = serde_json::from_slice(&wire).unwrap();
        assert_eq!(decoded.reply(&call).unwrap(), expected);
        let ordinary = Call::from_request(commands::request(Command::Get {
            address: fixtures::target().address,
        }));
        let mut forged = answer;
        forged.id = Some(ordinary.id.clone());
        assert_eq!(forged.reply(&ordinary), Err(PublicError::InvalidPayload));
    }
}

#[test]
fn domain_error_namespaces_and_closed_fields_cannot_be_swapped_or_extended() {
    let call = Call::from_request(commands::request(Command::ResolveConversation {
        binding: commands::resources::binding(),
    }));
    let hub = json!({(KIND):"hub",(CODE):"unauthorized"});
    let conversation = json!({(KIND):"conversation",(ERROR):ConversationError::BindingChanged});
    let mut extra = conversation.clone();
    extra[UNKNOWN_FIELD] = json!(true);
    let mut wrong_kind = conversation.clone();
    wrong_kind[KIND] = json!(MESSAGE);
    for (code, data) in [
        (CONVERSATION_ERROR, None),
        (HUB_ERROR, Some(Value::Null)),
        (CONVERSATION_ERROR, Some(hub.clone())),
        (HUB_ERROR, Some(conversation.clone())),
        (UNKNOWN_CODE, Some(hub)),
        (UNKNOWN_CODE, Some(conversation)),
        (CONVERSATION_ERROR, Some(extra)),
        (CONVERSATION_ERROR, Some(wrong_kind)),
    ] {
        let answer = Answer {
            jsonrpc: JSONRPC_VERSION.into(),
            id: Some(call.id.clone()),
            result: None,
            error: Some(RpcError {
                code,
                message: MESSAGE.into(),
                data,
            }),
        };
        assert_eq!(answer.reply(&call), Err(PublicError::InvalidPayload));
    }
}
