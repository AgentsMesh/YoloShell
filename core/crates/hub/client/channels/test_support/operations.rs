use crate::{harness::Harness, peer::*};
use std::time::Duration;
use tokio::time::timeout_at;
use uuid::Uuid;
use yolo_hub_client_channels::ControlEvent;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_hub_jsonrpc_protocol::Call as Request;
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, CreateShellResult, SecretCapability,
    TERMINAL_PROTOCOL_SCHEMA_VERSION, TerminalSize,
};

const CREATION_DIRECTORY: &str = "/tmp";
const NO_EXTRA_CONNECTION_WINDOW: Duration = Duration::from_millis(25);

pub fn create() -> Command {
    Command::Create {
        request: CreateShellRequest::new(CreateShellFields {
            schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
            request_id: Uuid::new_v4(),
            creation_token: Uuid::new_v4(),
            name: None,
            cwd: CREATION_DIRECTORY.into(),
            size: TerminalSize::default(),
        })
        .unwrap(),
    }
}

pub fn created(hub_id: HubId) -> Reply {
    Reply::Created {
        creation: CreateShellResult {
            address: TerminalAddress {
                hub_id,
                terminal_id: TerminalId::new(),
            },
            creation_capability: SecretCapability::generate(),
        },
    }
}

pub async fn dispatched(harness: &Harness, generation: Uuid) -> (Peer, Request) {
    let command = create();
    harness.send(Some(generation), command.clone()).await;
    let mut peer = harness.accept().await;
    let request = receive(&mut peer).await;
    assert_eq!(request.fields().command, command);
    (peer, request)
}

pub async fn awaiting_handshake(harness: &Harness) -> (Peer, Request) {
    let (stream, _) = timeout_at(deadline(), harness.listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut peer = Framed::new(stream);
    let request = receive(&mut peer).await;
    assert!(matches!(request.fields().command, Command::Hello {}));
    (peer, request)
}

pub async fn completed(
    harness: &mut Harness,
    generation: Uuid,
    command: &Command,
    expected: Reply,
) {
    let ControlEvent::Finished {
        request,
        result,
        connected,
    } = harness.event().await
    else {
        panic!("operation must complete without changing the control generation")
    };
    assert_eq!(request.generation, Some(generation));
    assert_eq!(&request.command, command);
    assert_eq!(result.unwrap(), expected);
    assert!(connected);
}

pub async fn round_trip(
    harness: &mut Harness,
    control: &mut Peer,
    generation: Uuid,
    command: Command,
    response: Reply,
) {
    harness.send(Some(generation), command.clone()).await;
    let request = receive(control).await;
    assert_eq!(request.fields().command, command);
    reply(control, &request, response.clone()).await;
    completed(harness, generation, &command, response).await;
}

pub async fn break_control(harness: &Harness, control: &mut Peer, generation: Uuid) {
    harness.send(Some(generation), Command::Hello {}).await;
    let request = receive(control).await;
    assert!(matches!(request.fields().command, Command::Hello {}));
    control.close(deadline()).await.unwrap();
}

pub async fn control_failed(harness: &mut Harness, generation: Uuid) {
    generation_retired(harness, generation).await;
    let ControlEvent::Finished {
        request,
        result: Err(_),
        connected: false,
    } = harness.event().await
    else {
        panic!("failed control probe must retire its generation")
    };
    assert_eq!(request.generation, Some(generation));
    assert!(matches!(request.command, Command::Hello {}));
}

pub async fn generation_retired(harness: &mut Harness, generation: Uuid) {
    // Authority retirement is published before the exact failed RPC result;
    // consumers must not continue admitting work while handling that result.
    let event = harness.event().await;
    assert!(
        matches!(event, ControlEvent::Unreachable { generation: Some(previous), .. }
            if previous == generation),
        "failed writer must first retire its exact generation: {event:?}"
    );
}

pub async fn cancel_without_replay(harness: &mut Harness) {
    harness.cancel().await;
    // After the owner has stopped, even a connection opened then immediately
    // dropped by an accidental replay remains observable in the listen queue.
    assert!(
        tokio::time::timeout(NO_EXTRA_CONNECTION_WINDOW, harness.listener.accept())
            .await
            .is_err(),
        "unexpected lifecycle connection or automatic replay"
    );
}
