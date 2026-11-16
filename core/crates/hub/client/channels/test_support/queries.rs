use crate::{harness::Harness, peer::*, query_requests};
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_jsonrpc_protocol::Call as Request;
use yolo_hub_rpc_protocol::Command;

pub async fn dispatched(harness: &Harness, generation: uuid::Uuid) -> (Peer, Request) {
    let command = query_requests::query();
    harness.send(Some(generation), command.clone()).await;
    let mut peer = harness.accept().await;
    let request = receive(&mut peer).await;
    assert_eq!(request.fields().command, command);
    (peer, request)
}

pub async fn failure(
    harness: &mut Harness,
    generation: uuid::Uuid,
    command: &Command,
    connected: bool,
) -> (ControlRequest, ControlError) {
    let ControlEvent::Finished {
        request,
        result: Err(error),
        connected: actual,
    } = harness.event().await
    else {
        panic!("query failure must retain its request without replacing fast control")
    };
    assert_eq!(request.generation, Some(generation));
    assert_eq!(&request.command, command);
    assert_eq!(actual, connected);
    (request, error)
}
