use crate::{harness::Harness, peer::*};
use std::time::Duration;
use tokio::time::{Instant, timeout_at};
use uuid::Uuid;
use yolo_hub_client_channels::ControlEvent;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, CreateShellResult, TERMINAL_PROTOCOL_SCHEMA_VERSION,
    SecretCapability, TerminalSize,
};

// A healthy shell startup may exceed the short inspection/control RPC budget.
// Virtual time makes that boundary deterministic without slowing down this test.
const DELAYED_CREATION: Duration = Duration::from_secs(6);
const CREATION_DIRECTORY: &str = "/tmp";

#[tokio::test]
async fn accepted_creation_waits_for_its_startup_result_beyond_the_inspection_deadline() {
    let mut harness = Harness::start();
    let mut control = accept(&harness).await;
    let generation = harness.ready().await;
    let command = Command::Create {
        request: CreateShellRequest::new(CreateShellFields {
            schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
            request_id: Uuid::new_v4(),
            creation_token: Uuid::new_v4(),
            name: None,
            cwd: CREATION_DIRECTORY.into(),
            size: TerminalSize::default(),
        })
        .unwrap(),
    };
    harness.send(Some(generation), command.clone()).await;
    // Lifecycle requests own their connection; fast control stays available
    // throughout the longer startup deadline.
    let mut operation = accept(&harness).await;
    let request = receive(&mut operation).await;
    assert_eq!(request.fields().command, command);
    let creation = CreateShellResult {
        address: TerminalAddress {
            hub_id: harness.endpoint.hub,
            terminal_id: TerminalId::new(),
        },
        creation_capability: SecretCapability::generate(),
    };
    tokio::time::pause();
    let awaiting_result = Instant::now();
    tokio::select! {
        event = harness.event() => panic!(
            "accepted shell startup was abandoned before its result: {event:?}"
        ),
        _ = tokio::time::sleep(DELAYED_CREATION) => {},
    }
    assert!(awaiting_result.elapsed() >= DELAYED_CREATION);
    tokio::time::resume();
    reply(&mut operation, &request, Reply::Created { creation }).await;
    let ControlEvent::Finished {
        request: completed,
        result,
        connected,
    } = harness.event().await
    else {
        panic!("creation must have exactly one correlated completion")
    };
    assert_eq!(completed.generation, Some(generation));
    assert_eq!(completed.command, command);
    assert!(matches!(result, Ok(Reply::Created { creation: actual }) if actual == creation));
    assert!(connected);
    harness.finish().await;
    // Any replay, including one queued while shutting down, is a protocol frame
    // rather than EOF and fails these assertions.
    eof(&mut control).await;
    eof(&mut operation).await;
}

async fn accept(harness: &Harness) -> Peer {
    let (stream, _) = timeout_at(deadline(), harness.listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut peer = Framed::new(stream);
    handshake(&mut peer, harness.endpoint.hub).await;
    peer
}

async fn handshake(peer: &mut Peer, hub: HubId) {
    let request = receive(peer).await;
    assert!(matches!(request.fields().command, Command::Hello {}));
    reply(peer, &request, hello(hub)).await;
}

fn hello(hub_id: HubId) -> Reply {
    Reply::Hello {
        hello: Hello {
            hub_id,
            version: HUB_PROTOCOL_VERSION,
            methods: vec![Method::Hello, Method::Create],
        },
    }
}
