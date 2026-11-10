use crate::{commands::grant, harness::Harness, ordered_inputs::*, peer::*, scoped_requests::send};
use uuid::Uuid;
use yolo_hub_client_channels::{ControlError, ControlEvent, RequestScope};
use yolo_hub_jsonrpc_transport::Framed;

#[tokio::test]
async fn queued_input_bytes_are_coalesced_into_one_wire_request_and_two_receipts() {
    let mut harness = Harness::start();
    let mut peer = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let scope = RequestScope::ordered_inputs();
    let prefix = input(grant, PREFIX);
    let suffix = input(grant, SUFFIX);
    send(&harness, generation, prefix.clone(), validity(&scope)).await;
    send(&harness, generation, suffix.clone(), validity(&scope)).await;
    let dispatched = receive(&mut peer).await;
    let yolo_hub_rpc_protocol::Command::Input { request } = &dispatched.fields().command else {
        panic!()
    };
    let expected = [PREFIX, SUFFIX].concat();
    assert_eq!(request.fields().data, expected);
    reply(
        &mut peer,
        &dispatched,
        yolo_hub_rpc_protocol::Reply::Input { duplicate: false },
    )
    .await;
    for expected_command in [prefix, suffix] {
        let ControlEvent::Finished {
            request,
            result: Ok(_),
            connected: true,
        } = harness.event().await
        else {
            panic!()
        };
        assert_eq!(request.command, expected_command);
    }
    harness.finish().await;
    eof(&mut peer).await;
}

#[tokio::test]
async fn pre_ready_rejection_cancels_the_input_group_before_a_fresh_generation_can_admit_suffix() {
    let mut harness = Harness::start();
    let (stream, _) = tokio::time::timeout_at(deadline(), harness.listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut peer = Framed::new(stream);
    let handshake = receive(&mut peer).await;
    let grant = grant(harness.endpoint.hub);
    let scope = RequestScope::ordered_inputs();
    let prefix = input(grant, PREFIX);
    send(&harness, Uuid::new_v4(), prefix.clone(), validity(&scope)).await;
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::NotConnected),
        connected: false,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(request.command, prefix);
    assert!(!validity(&scope).is_current());
    reply(&mut peer, &handshake, hello_reply(harness.endpoint.hub)).await;
    let generation = harness.ready().await;
    let suffix = input(grant, SUFFIX);
    send(&harness, generation, suffix.clone(), validity(&scope)).await;
    wire_barrier(&harness, &mut peer, generation).await;
    rejected(&mut harness, &suffix).await;
    completed_barrier(&mut harness).await;
    harness.finish().await;
    eof(&mut peer).await;
}
