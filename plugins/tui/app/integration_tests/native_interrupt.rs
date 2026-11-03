use super::{channel_peer::*, constants::*, interrupt_support::*};
use crate::{
    Options, channel_network,
    constants::QUEUE_CAPACITY,
    events,
    interrupt_actions::status,
    interrupt_constants,
    network::{Job, Update},
};
use crossterm::event::KeyEventKind;
use tokio::{net::UnixListener, sync::mpsc, time::timeout_at};
use yolo_hub_agent_protocol::{AgentAction, CommandOutcome};
use yolo_hub_client::HubEndpoint;
use yolo_hub_client_channels::ControlEvent;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

#[tokio::test]
async fn inventory_interrupt_traverses_existing_entry_control_channel_and_unknown_is_never_replayed()
 {
    let directory = tempfile::tempdir_in(NETWORK_TEST_ROOT).unwrap();
    let socket = directory.path().join(NETWORK_TEST_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let entry = HubId::new();
    let (jobs, incoming) = mpsc::channel(QUEUE_CAPACITY);
    let (updates, mut responses) = mpsc::channel(QUEUE_CAPACITY);
    let worker = tokio::spawn(channel_network::network(
        Options {
            endpoint: HubEndpoint::local(socket),
            hub: entry,
            layout: None,
        },
        incoming,
        updates,
    ));
    let mut worker = AbortOnDrop(worker);
    let (stream, _) = timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut peer = Framed::new(stream);
    let handshake = receive(&mut peer).await;
    assert!(matches!(handshake.fields().command, Command::Hello {}));
    respond(&mut peer, &handshake, hello_reply(entry)).await;
    let Some(Update::Control(ControlEvent::Ready { generation, .. })) =
        timeout_at(deadline(), responses.recv()).await.unwrap()
    else {
        panic!()
    };
    let mut state = ready();
    state.control_generation = Some(generation);
    let selected = target(&state);
    assert_ne!(entry, selected.address.hub_id);
    press(&mut state, &jobs, KeyEventKind::Press);
    press(&mut state, &jobs, KeyEventKind::Press);
    let sent = receive(&mut peer).await;
    assert_eq!(sent.fields().destination, selected.address.hub_id);
    let Command::AgentControl { request, .. } = &sent.fields().command else {
        panic!()
    };
    assert_eq!(request.fields().target, selected);
    assert_eq!(request.fields().action, AgentAction::Interrupt {});
    let mut receipt =
        yolo_hub_agent_protocol::CommandReceipt::accepted(request.fields().request_id, selected)
            .unwrap();
    receipt.advance(CommandOutcome::Unknown {}).unwrap();
    respond(&mut peer, &sent, Reply::AgentControl { receipt }).await;
    let update = timeout_at(deadline(), responses.recv())
        .await
        .unwrap()
        .unwrap();
    events::apply(&mut state, &jobs, update);
    assert_eq!(status(&state), interrupt_constants::UNKNOWN);
    press(&mut state, &jobs, KeyEventKind::Repeat);
    press(&mut state, &jobs, KeyEventKind::Press);
    // A read-only barrier after the repeated keys proves no second mutation was queued.
    jobs.send(Job::Call {
        generation: Some(generation),
        origin: entry,
        command: Command::Hello {},
    })
    .await
    .ok()
    .unwrap();
    let barrier = receive(&mut peer).await;
    assert!(matches!(barrier.fields().command, Command::Hello {}));
    respond(&mut peer, &barrier, hello_reply(entry)).await;
    worker.0.abort();
    assert!((&mut worker.0).await.unwrap_err().is_cancelled());
}

fn hello_reply(hub_id: HubId) -> Reply {
    Reply::Hello {
        hello: Hello {
            hub_id,
            version: HUB_PROTOCOL_VERSION,
            methods: vec![Method::Hello, Method::List, Method::AgentControl],
        },
    }
}
struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
