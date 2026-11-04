use crate::{
    harness::Harness,
    peer::{Peer, receive, reply},
};
use uuid::Uuid;
use yolo_hub_client_channels::{ControlEvent, ControlRequest, RequestValidity};
use yolo_hub_rpc_protocol::{Command, Reply};

pub async fn send(
    harness: &Harness,
    generation: Uuid,
    command: Command,
    validity: RequestValidity,
) {
    harness
        .commands
        .as_ref()
        .unwrap()
        .send(ControlRequest {
            generation: Some(generation),
            destination: harness.endpoint.hub,
            command,
            validity: Some(validity),
        })
        .await
        .unwrap();
}

pub async fn delivered(harness: &mut Harness, expected: &Command) {
    let ControlEvent::Finished {
        request,
        result,
        connected,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(&request.command, expected);
    assert!(connected);
    assert!(matches!(result, Ok(Reply::Input { duplicate: false })));
}

pub async fn expect_current(harness: &mut Harness, peer: &mut Peer, expected: &Command) {
    // Because the transport is sequential, this exact next frame also proves
    // that no retired queued request was written ahead of the current command.
    let received = receive(peer).await;
    assert_eq!(&received.fields().command, expected);
    reply(peer, &received, Reply::Input { duplicate: false }).await;
    delivered(harness, expected).await;
}
