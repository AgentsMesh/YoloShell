use crate::{fixture::*, peer::*};
use yolo_conversation_protocol::ConversationError;
use yolo_hub_conversation_client::*;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn current_resolution_uses_the_authoritative_execution_once() {
    let conversation = conversation();
    let address = conversation.binding.agent.address;
    let (mut client, server) = connected(vec![
        Reply::Agent {
            agent: agent(&conversation),
        },
        Reply::Conversation {
            conversation: conversation.clone(),
        },
    ])
    .await;
    assert_eq!(
        resolve_current(&mut client, address, deadline())
            .await
            .unwrap(),
        conversation
    );
    assert_eq!(
        server.await.unwrap(),
        vec![
            Command::GetCurrentAgent { address },
            Command::ResolveConversation {
                binding: conversation.binding
            },
        ]
    );
}

#[tokio::test]
async fn resolve_preflight_rejects_cross_origin_binding_without_a_request() {
    let (mut client, server) = connected(vec![]).await;
    let mut binding = conversation().binding;
    binding.runtime.address.hub_id = HubId::new();
    assert!(matches!(
        resolve(&mut client, binding, deadline()).await,
        Err(ConversationClientError::Conversation(
            ConversationError::InvalidRequest
        ))
    ));
    assert!(server.await.unwrap().is_empty());
}
