use super::{
    super::federation_support::{RunningNode, deadline},
    constants::*,
};
use serde_json::{Value, json};
use tokio::net::UnixStream;
use yolo_conversation_protocol::ConversationReadRequest;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{Answer, CHAT_METHOD, Call};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::Command;

async fn call(stream: &mut Framed<UnixStream>, message: Value) -> Value {
    stream.send(&message, deadline()).await.unwrap();
    let Incoming::Message(answer) = stream.receive::<Value>(deadline()).await.unwrap() else {
        panic!("Hub closed the chat connection")
    };
    assert_eq!(answer["id"], message["id"]);
    answer
}

pub(super) async fn read(
    entry: &RunningNode,
    entry_id: HubId,
    request: &ConversationReadRequest,
) -> Value {
    let mut stream = Framed::new(UnixStream::connect(&entry.socket).await.unwrap());
    let hello = Call::new(entry_id, Command::Hello {}).unwrap();
    stream.send(&hello, deadline()).await.unwrap();
    let Incoming::Message(answer) = stream.receive::<Answer>(deadline()).await.unwrap() else {
        panic!("Hub closed before hello")
    };
    answer.reply(&hello).unwrap();
    call(
        &mut stream,
        json!({
            "jsonrpc":"2.0","id":"read-through-entry","method": CHAT_METHOD,
            "params":{"destination":request.address.hub_id,"operation":"read","request":request}
        }),
    )
    .await
}

/// An authorized mutation must reach the origin execution lookup, including
/// for an execution that disappeared between the catalogue and submission.
pub(super) async fn verify_origin_prompt(entry: &RunningNode, request: &ConversationReadRequest) {
    use yolo_hub_agent_protocol::{
        AGENT_PROTOCOL_SCHEMA_VERSION, AgentPromptFields, AgentPromptReference, AgentPromptRequest,
        AgentPromptWait, Prompt,
    };
    let mut retired = request.binding.agent;
    retired.execution_id = yolo_hub_identity::ExecutionId::new();
    let prompt = AgentPromptRequest::new(AgentPromptFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        reference: AgentPromptReference::Execution { target: retired },
        prompt: Prompt::new(CONTENT_MARKER.into()).unwrap(),
        wait: AgentPromptWait::None,
        take_control: false,
    })
    .unwrap();
    let result = entry
        .client()
        .await
        .call(
            request.address.hub_id,
            Command::AgentPrompt { request: prompt },
            deadline(),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(yolo_hub_client::ClientError::Remote(
                yolo_hub_terminal_protocol::PublicError::NotFound
            ))
        ),
        "origin execution lookup: {result:?}"
    );
}

pub(super) fn verify(page: &Value, session: &str) {
    assert_eq!(page["sessionId"], session);
    assert!(
        page["entries"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|row| row["updates"].as_array().unwrap())
            .any(|update| update["sessionUpdate"] == AGENT_MESSAGE_CHUNK
                && update["content"]["text"] == CONTENT_MARKER)
    );
}
