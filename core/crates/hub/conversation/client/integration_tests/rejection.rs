use crate::constants::INVALID_BYTE_LIMIT;
use crate::{fixture::*, peer::*};
use yolo_conversation_protocol::*;
use yolo_hub_client::ClientError;
use yolo_hub_conversation_client::*;
use yolo_hub_identity::{ExecutionId, HubId};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn native_history_failures_remain_typed_and_never_trigger_retries() {
    let conversation = conversation();
    for is_resolve in [true, false] {
        let (mut client, server) = connected(vec![Reply::ConversationUnavailable {
            error: ConversationError::HistoryChanged,
        }])
        .await;
        let error = if is_resolve {
            resolve(&mut client, conversation.binding, deadline())
                .await
                .unwrap_err()
        } else {
            read(&mut client, request(&conversation), deadline())
                .await
                .unwrap_err()
        };
        assert!(matches!(
            error,
            ConversationClientError::Conversation(ConversationError::HistoryChanged)
        ));
        assert_eq!(server.await.unwrap().len(), 1);
    }
    let (mut client, server) = connected(vec![Reply::Error {
        code: PublicError::Unauthorized,
    }])
    .await;
    assert!(matches!(
        watch(&mut client, request(&conversation), 0, deadline()).await,
        Err(ConversationClientError::Client(ClientError::Remote(
            PublicError::Unauthorized
        )))
    ));
    assert_eq!(server.await.unwrap().len(), 1);
}

#[tokio::test]
async fn wrong_execution_or_cross_origin_reply_cannot_rebind_the_reader() {
    let mut conversation = conversation();
    let target = conversation.binding.agent;
    conversation.binding.agent.execution_id = ExecutionId::new();
    let (mut client, server) = connected(vec![Reply::Agent {
        agent: agent(&conversation),
    }])
    .await;
    assert!(matches!(
        resolve_agent(&mut client, target, deadline()).await,
        Err(ConversationClientError::Client(ClientError::InvalidResponse(_)))
    ));
    assert_eq!(server.await.unwrap().len(), 1);
    let binding = conversation.binding;
    conversation.address.hub_id = HubId::new();
    let (mut client, server) = connected(vec![Reply::Conversation { conversation }]).await;
    assert!(matches!(
        resolve(&mut client, binding, deadline()).await,
        Err(ConversationClientError::Client(
            ClientError::InvalidResponse(_)
        ))
    ));
    assert!(!client.is_usable());
    assert_eq!(server.await.unwrap().len(), 1);
}

#[tokio::test]
async fn malformed_preflight_and_forged_unchanged_generation_are_rejected() {
    let conversation = conversation();
    let (mut client, server) = connected(Vec::new()).await;
    let mut invalid = request(&conversation);
    invalid.limits.bytes = INVALID_BYTE_LIMIT;
    assert!(matches!(
        read(&mut client, invalid.clone(), deadline()).await,
        Err(ConversationClientError::Conversation(
            ConversationError::InvalidRequest
        ))
    ));
    assert!(matches!(
        watch(&mut client, invalid, 0, deadline()).await,
        Err(ConversationClientError::Conversation(
            ConversationError::InvalidRequest
        ))
    ));
    assert!(matches!(
        watch(
            &mut client,
            request(&conversation),
            MAX_WATCH_WAIT_MS + 1,
            deadline()
        )
        .await,
        Err(ConversationClientError::Conversation(
            ConversationError::InvalidRequest
        ))
    ));
    assert!(server.await.unwrap().is_empty());
    let page = page(conversation.clone());
    let mut request = request(&conversation);
    request.since_revision = Some(page.revision);
    request.since_generation = Some(page.history_generation);
    let (mut client, server) = connected(vec![Reply::ConversationRead {
        reading: ConversationReadResult::Unchanged {
            history_generation: yolo_crypto::Sha256Digest::repeated(9),
            revision: page.revision,
        },
    }])
    .await;
    assert!(matches!(
        watch(&mut client, request, 0, deadline()).await,
        Err(ConversationClientError::Client(
            ClientError::InvalidResponse(_)
        ))
    ));
    assert_eq!(server.await.unwrap().len(), 1);
}
