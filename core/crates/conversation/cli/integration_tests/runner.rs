use super::{fixture::*, peer::peer};
use crate::{arguments::Action, runner::run};
use yolo_conversation_protocol::ConversationError;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Reply;

#[tokio::test]
async fn current_agent_commands_use_public_discovery_and_never_native_paths() {
    let entry = HubId::new();
    let origin = HubId::new();
    let descriptor = descriptor(origin);
    for read in [false, true] {
        let mut replies = vec![
            Reply::Agent {
                agent: agent(&descriptor),
            },
            Reply::Conversation {
                conversation: descriptor.clone(),
            },
        ];
        if read {
            replies.push(Reply::ConversationRead {
                reading: reading(&descriptor),
            });
        }
        let action = if read {
            Action::Read {
                agent: Some(descriptor.binding.agent.address.agent_id),
                origin: Some(origin),
            }
        } else {
            Action::Resolve {
                agent: descriptor.binding.agent.address.agent_id,
                origin: Some(origin),
            }
        };
        let expected = replies.last().unwrap().clone();
        let (options, _directory, server) = peer(entry, action, replies);
        assert_eq!(run(options, None).await.unwrap(), expected);
        assert_eq!(server.await.unwrap().len(), if read { 3 } else { 2 });
    }
}

#[tokio::test]
async fn typed_read_and_watch_preserve_machine_readable_failures() {
    for watch in [false, true] {
        let hub = HubId::new();
        let descriptor = descriptor(hub);
        let action = if watch {
            Action::Watch { wait_ms: 0 }
        } else {
            Action::Read {
                agent: None,
                origin: None,
            }
        };
        let expected = Reply::ConversationUnavailable {
            error: ConversationError::HistoryChanged,
        };
        let (options, _directory, server) = peer(hub, action, vec![expected.clone()]);
        assert_eq!(
            run(options, Some(request(&descriptor))).await.unwrap(),
            expected
        );
        assert_eq!(server.await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn transport_failure_is_not_reported_as_an_empty_conversation() {
    let hub = HubId::new();
    let descriptor = descriptor(hub);
    let (options, _directory, server) = peer(
        hub,
        Action::Read {
            agent: None,
            origin: None,
        },
        Vec::new(),
    );
    assert!(run(options, Some(request(&descriptor))).await.is_err());
    assert!(server.await.unwrap().is_empty());
}
