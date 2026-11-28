use super::{channel_peer::*, chat_support::*, constants::*, interrupt_support};
use crate::{
    Options, channel_network, events,
    network::{Job, Update},
};
use crossterm::event::{KeyCode, KeyModifiers};
use tokio::{net::UnixListener, sync::mpsc, time::timeout_at};
use yolo_hub_agent_protocol::CommandOutcome;
use yolo_hub_client::HubEndpoint;
use yolo_hub_client_channels::ControlEvent;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

fn greeting(hub_id: HubId) -> Reply {
    Reply::Hello {
        hello: Hello {
            hub_id,
            version: HUB_PROTOCOL_VERSION,
            methods: vec![
                Method::Hello,
                Method::AgentControl,
                Method::ResolveConversation,
            ],
        },
    }
}

#[tokio::test]
async fn stalled_chat_reader_does_not_block_semantic_submit_and_unknown_never_replays() {
    let root = tempfile::tempdir_in(NETWORK_TEST_ROOT).unwrap();
    let socket = root.path().join(NETWORK_TEST_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let hub = HubId::new();
    let (jobs, incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let (updates, mut responses) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let task = tokio::spawn(channel_network::network(
        Options {
            endpoint: HubEndpoint::local(socket),
            hub,
            layout: None,
        },
        incoming,
        updates,
    ));
    let task = AbortOnDrop(task);
    let (stream, _) = timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut control = Framed::new(stream);
    let hello = receive(&mut control).await;
    respond(&mut control, &hello, greeting(hub)).await;
    let Update::Control(ControlEvent::Ready { generation, .. }) = responses.recv().await.unwrap()
    else {
        panic!()
    };
    let (mut state, _, _) = ready();
    state.control_generation = Some(generation);
    toggle(&mut state, &jobs);
    crate::chat::pump(&mut state, &jobs);
    let (stream, _) = timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut query = Framed::new(stream);
    let hello = receive(&mut query).await;
    respond(&mut query, &hello, greeting(hub)).await;
    let pending_read = receive(&mut query).await;
    assert!(matches!(
        pending_read.fields().command,
        Command::ResolveConversation { .. }
    ));
    // Leave the reader unresolved; the writer lane must still deliver its own request.
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let submission = receive(&mut control).await;
    let Command::AgentControl { request, .. } = &submission.fields().command else {
        panic!()
    };
    assert_eq!(
        request.fields().target,
        crate::chat::current(&state).unwrap().binding().agent
    );
    let mut receipt = yolo_hub_agent_protocol::CommandReceipt::accepted(
        request.fields().request_id,
        request.fields().target,
    )
    .unwrap();
    receipt.advance(CommandOutcome::Unknown {}).unwrap();
    respond(&mut control, &submission, Reply::AgentControl { receipt }).await;
    let update = timeout_at(deadline(), responses.recv())
        .await
        .unwrap()
        .unwrap();
    events::apply(&mut state, &jobs, update);
    assert_eq!(
        crate::chat::current(&state).unwrap().composer.phase(),
        Some(yolo_tui_chat::SendPhase::Unknown)
    );
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    jobs.send(Job::Call {
        generation: Some(generation),
        origin: hub,
        command: Command::Hello {},
    })
    .await
    .ok()
    .unwrap();
    let barrier = receive(&mut control).await;
    assert!(matches!(barrier.fields().command, Command::Hello {}));
    respond(&mut control, &barrier, greeting(hub)).await;
    // Leaving Chat invalidates this query before its delayed provider-unavailable response.
    key(&mut state, &jobs, KeyCode::Esc, KeyModifiers::NONE);
    crate::chat::synchronize(&mut state);
    respond(
        &mut query,
        &pending_read,
        Reply::ConversationUnavailable {
            error: yolo_conversation_protocol::ConversationError::EvidenceUnavailable,
        },
    )
    .await;
    loop {
        let update = timeout_at(deadline(), responses.recv())
            .await
            .unwrap()
            .unwrap();
        let matched = matches!(update, Update::Chat { .. });
        events::apply(&mut state, &jobs, update);
        if matched {
            break;
        }
    }
    assert!(crate::chat::current(&state).unwrap().error().is_none());
    assert!(
        state
            .writers
            .contains_key(&state.focused_terminal().unwrap())
    );
    drop(task);
}
struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
