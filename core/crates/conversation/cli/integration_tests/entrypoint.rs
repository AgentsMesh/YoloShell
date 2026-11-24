use super::{constants::*, fixture::*, peer::peer};
use crate::{
    arguments::Action,
    constants::{COMMAND, PROGRAM},
};
use std::ffi::OsString;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Reply;

#[tokio::test]
async fn actual_cli_entrypoint_reads_current_agent_on_an_existing_remote_origin() {
    for read in [false, true] {
        let entry = HubId::new();
        let origin = HubId::new();
        let descriptor = descriptor(origin);
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
        let (options, _directory, server) = peer(
            entry,
            Action::Resolve {
                agent: descriptor.binding.agent.address.agent_id,
                origin: Some(origin),
            },
            replies,
        );
        let arguments = [
            PROGRAM.to_owned(),
            COMMAND.to_owned(),
            HUB_OPTION.to_owned(),
            entry.to_string(),
            SOCKET_OPTION.to_owned(),
            options.socket.as_ref().unwrap().to_str().unwrap().to_owned(),
            if read { READ } else { RESOLVE }.to_owned(),
            AGENT_OPTION.to_owned(),
            descriptor.binding.agent.address.agent_id.to_string(),
            ORIGIN_OPTION.to_owned(),
            origin.to_string(),
        ]
        .map(OsString::from);
        tokio::task::spawn_blocking(move || crate::run_from(arguments))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            server.await.unwrap().len(),
            if read {
                READ_REQUEST_COUNT
            } else {
                RESOLVE_REQUEST_COUNT
            }
        );
    }
}

#[tokio::test]
async fn absent_typed_input_never_dispatches_a_read_or_watch_after_hello() {
    for action in [
        Action::Read {
            agent: None,
            origin: None,
        },
        Action::Watch { wait_ms: 0 },
    ] {
        let (options, _directory, server) = peer(HubId::new(), action, vec![]);
        let reply = crate::runner::run(options, None).await.unwrap();
        assert!(matches!(
            reply,
            Reply::ConversationUnavailable {
                error: yolo_conversation_protocol::ConversationError::InvalidRequest,
            }
        ));
        assert!(server.await.unwrap().is_empty());
    }
}
