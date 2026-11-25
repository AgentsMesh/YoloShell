use super::{channel_peer::*, chat_projection::*, chat_support::*, constants::*};
use crate::{
    Options,
    chat::{
        self,
        network::{self, Operation, Reading},
    },
    network::Update,
};
use tokio::{net::UnixListener, sync::mpsc, time::timeout_at};
use yolo_conversation_protocol::*;
use yolo_hub_client::HubEndpoint;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};
use yolo_tui_chat::QueryError;

const PAGE_OFFSET: u64 = 100;

#[tokio::test]
async fn real_conversation_reader_resolves_reads_watches_and_paginates_on_its_own_channels() {
    let root = tempfile::tempdir_in(NETWORK_TEST_ROOT).unwrap();
    let socket = root.path().join(NETWORK_TEST_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    let descriptor = descriptor(&state);
    let hub = descriptor.address.hub_id;
    let newest = page(&descriptor, 2, PAGE_OFFSET);
    let old = page(&descriptor, 1, PAGE_OFFSET - 1);
    let expected = descriptor.clone();
    let server = tokio::spawn(async move {
        for stage in 0..4 {
            let (stream, _) = timeout_at(deadline(), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut peer = yolo_hub_jsonrpc_transport::Framed::new(stream);
            let hello = receive(&mut peer).await;
            respond(
                &mut peer,
                &hello,
                Reply::Hello {
                    hello: Hello {
                        hub_id: hub,
                        version: HUB_PROTOCOL_VERSION,
                        methods: vec![
                            Method::Hello,
                            Method::ResolveConversation,
                            Method::ReadConversation,
                            Method::WatchConversation,
                        ],
                    },
                },
            )
            .await;
            let request = receive(&mut peer).await;
            let reply = match stage {
                0 => {
                    assert!(matches!(
                        request.fields().command,
                        Command::ResolveConversation { .. }
                    ));
                    Reply::Conversation {
                        conversation: expected.clone(),
                    }
                }
                1 => {
                    assert!(matches!(
                        request.fields().command,
                        Command::ReadConversation { .. }
                    ));
                    Reply::ConversationRead {
                        reading: ConversationReadResult::Page {
                            page: newest.clone(),
                        },
                    }
                }
                2 => {
                    assert!(matches!(
                        request.fields().command,
                        Command::WatchConversation { .. }
                    ));
                    Reply::ConversationRead {
                        reading: ConversationReadResult::Unchanged {
                            history_generation: newest.history_generation,
                            revision: newest.revision,
                        },
                    }
                }
                _ => {
                    assert!(matches!(
                        request.fields().command,
                        Command::ReadConversation { .. }
                    ));
                    Reply::ConversationRead {
                        reading: ConversationReadResult::Page { page: old.clone() },
                    }
                }
            };
            respond(&mut peer, &request, reply).await;
        }
    });
    let options = Options {
        endpoint: HubEndpoint::local(socket),
        hub,
        layout: None,
    };
    for stage in 0..4 {
        if stage == 3 {
            let key = chat::Key::current(&state);
            state.chats.entries.get_mut(&key).unwrap().older = true;
        }
        let query = operation(&mut state, &jobs, &mut incoming);
        let update = execute(&options, query).await;
        crate::events::apply(&mut state, &jobs, update);
    }
    assert_eq!(chat::current(&state).unwrap().entries().len(), 2);
    assert!(chat::current(&state).unwrap().error().is_none());
    server.await.unwrap();
}

#[tokio::test]
async fn missing_socket_and_unsupported_reader_have_distinct_errors_and_send_no_query() {
    let root = tempfile::tempdir_in(NETWORK_TEST_ROOT).unwrap();
    let socket = root.path().join(NETWORK_TEST_SOCKET);
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    let operation = operation(&mut state, &jobs, &mut incoming);
    let options = Options {
        endpoint: HubEndpoint::local(socket.clone()),
        hub: state.focused_terminal().unwrap().hub_id,
        layout: None,
    };
    let Update::Chat { result, .. } = execute(&options, operation.clone()).await else {
        panic!()
    };
    assert!(matches!(result, Err(QueryError::Disconnected)));
    let listener = UnixListener::bind(&socket).unwrap();
    let hub = options.hub;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut peer = yolo_hub_jsonrpc_transport::Framed::new(stream);
        let hello = receive(&mut peer).await;
        respond(
            &mut peer,
            &hello,
            Reply::Hello {
                hello: Hello {
                    hub_id: hub,
                    version: HUB_PROTOCOL_VERSION,
                    methods: vec![Method::Hello],
                },
            },
        )
        .await;
        assert!(matches!(
            peer.receive::<yolo_hub_jsonrpc_protocol::Call>(deadline())
                .await
                .unwrap(),
            yolo_hub_jsonrpc_transport::Incoming::Eof
        ));
    });
    let Update::Chat { result, .. } = execute(&options, operation.clone()).await else {
        panic!()
    };
    assert!(matches!(result, Err(QueryError::Unsupported)));
    server.await.unwrap();
    let (sender, receiver) = mpsc::channel(1);
    let (updates, output) = mpsc::channel(1);
    drop(output);
    sender.send(operation).await.unwrap();
    drop(sender);
    network::run(&options, receiver, &updates).await;
}

async fn execute(options: &Options, operation: Operation) -> Update {
    let (sender, receiver) = mpsc::channel(1);
    let (updates, mut output) = mpsc::channel(1);
    sender.send(operation).await.unwrap();
    drop(sender);
    network::run(options, receiver, &updates).await;
    timeout_at(deadline(), output.recv())
        .await
        .unwrap()
        .unwrap()
}
