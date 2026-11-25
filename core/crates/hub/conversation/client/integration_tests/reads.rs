use crate::{fixture::*, peer::*};
use yolo_conversation_protocol::*;
use yolo_hub_conversation_client::*;
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn resolve_discovers_runtime_and_reads_through_the_target_origin_only() {
    let conversation = conversation();
    let page = page(conversation.clone());
    let request = request(&conversation);
    let (mut client, server) = connected(vec![
        Reply::Agent {
            agent: agent(&conversation),
        },
        Reply::Conversation {
            conversation: conversation.clone(),
        },
        Reply::ConversationRead {
            reading: ConversationReadResult::Page { page: page.clone() },
        },
    ])
    .await;
    assert_eq!(
        resolve_agent(&mut client, conversation.binding.agent, deadline())
            .await
            .unwrap(),
        conversation
    );
    assert_eq!(
        read(&mut client, request.clone(), deadline())
            .await
            .unwrap(),
        ConversationReadResult::Page { page }
    );
    assert_eq!(
        server.await.unwrap(),
        vec![
            Command::GetAgent {
                target: conversation.binding.agent
            },
            Command::ResolveConversation {
                binding: conversation.binding
            },
            Command::ReadConversation { request },
        ]
    );
}

#[tokio::test]
async fn bounded_watch_preserves_exact_conditional_generation_and_revision() {
    let conversation = conversation();
    let page = page(conversation.clone());
    let mut request = request(&conversation);
    request.since_revision = Some(page.revision);
    request.since_generation = Some(page.history_generation);
    let unchanged = ConversationReadResult::Unchanged {
        history_generation: page.history_generation,
        revision: page.revision,
    };
    let (mut client, server) = connected(vec![Reply::ConversationRead {
        reading: unchanged.clone(),
    }])
    .await;
    assert_eq!(
        watch(&mut client, request.clone(), MAX_WATCH_WAIT_MS, deadline())
            .await
            .unwrap(),
        unchanged
    );
    assert_eq!(
        server.await.unwrap(),
        vec![Command::WatchConversation {
            request,
            wait_ms: MAX_WATCH_WAIT_MS
        }]
    );
}
