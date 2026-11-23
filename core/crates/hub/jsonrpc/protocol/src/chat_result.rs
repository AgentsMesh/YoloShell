use crate::{Answer, Call, JSONRPC_VERSION};
use serde_json::{Value, json};
use yolo_conversation_protocol::ConversationReadResult;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

pub(super) fn encode(call: &Call, reply: Reply) -> Result<Answer, PublicError> {
    let body = match reply {
        Reply::Conversation { conversation } => json!({
            "address": conversation.address,
            "binding": conversation.binding,
            "sessionId": conversation.address.chat_session_id(),
        }),
        Reply::ConversationRead { reading } => match reading {
            ConversationReadResult::Page { page } => json!({
                "reading": { "status": "page", "page": page.chat_page() },
            }),
            unchanged => json!({ "reading": unchanged }),
        },
        Reply::Agent { agent } => json!({ "agent": agent }),
        Reply::AgentPrompt { prompt } => json!({ "prompt": prompt }),
        Reply::AgentControl { receipt } => json!({ "receipt": receipt }),
        error @ (Reply::Error { .. } | Reply::ConversationUnavailable { .. }) => {
            return Answer::new(call, error);
        }
        _ => return Err(PublicError::InvalidPayload),
    };
    let Value::Object(mut result) = body else {
        unreachable!()
    };
    result.insert("version".into(), Value::from(crate::CHAT_VERSION));
    Ok(Answer {
        jsonrpc: JSONRPC_VERSION.into(),
        id: Some(call.id.clone()),
        result: Some(Value::Object(result)),
        error: None,
    })
}
