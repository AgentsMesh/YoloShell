use super::{
    channel_peer::*,
    constants::QUEUE_OVERFLOW,
    input_transport::{Harness, hello_reply},
};
use crate::constants::QUEUE_CAPACITY;
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_rpc_protocol::{Command, Reply};

const INPUT_PREFIX: &[u8] = b"printf '%s\\n' '";
const INPUT_SUFFIX: &[u8] = b"'\r";
const PRESSURE_BYTE: u8 = b'x';

async fn finish_hello(harness: &mut Harness, request: &yolo_hub_jsonrpc_protocol::Call) {
    respond(
        &mut harness.peer,
        request,
        hello_reply(request.fields().destination),
    )
    .await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Ok(Reply::Hello { .. }),
            ..
        }
    ));
}

#[tokio::test]
async fn a_saturated_control_queue_delivers_every_input_once_in_order_including_the_final_quote() {
    let mut harness = Harness::start().await;
    harness.send_hello().await;
    let blocked = receive(&mut harness.peer).await;
    let bytes = [
        INPUT_PREFIX,
        &vec![PRESSURE_BYTE; QUEUE_CAPACITY + QUEUE_OVERFLOW],
        INPUT_SUFFIX,
    ]
    .concat();
    let mut requests = Vec::new();
    for byte in &bytes {
        requests.push(harness.send_input(vec![*byte]).await);
    }
    // The outer queue cannot be fully drained when more than one request is
    // waiting behind the saturated inner queue; inspect every completion below.
    finish_hello(&mut harness, &blocked).await;
    let mut delivered = Vec::new();
    let mut wire_requests = Vec::new();
    while delivered.len() < bytes.len() {
        let request = receive(&mut harness.peer).await;
        let Command::Input { request: input } = &request.fields().command else {
            panic!()
        };
        delivered.extend_from_slice(&input.fields().data);
        wire_requests.push(request);
    }
    assert_eq!(delivered, bytes);
    for request in wire_requests {
        respond(
            &mut harness.peer,
            &request,
            Reply::Input { duplicate: false },
        )
        .await;
    }
    for expected in requests {
        let ControlEvent::Finished {
            request,
            result: Ok(Reply::Input { duplicate: false }),
            connected: true,
        } = harness.event().await
        else {
            panic!()
        };
        assert_eq!(request.command, expected.command);
    }
    harness.send_hello().await;
    let after = receive(&mut harness.peer).await;
    assert!(matches!(after.fields().command, Command::Hello {}));
    finish_hello(&mut harness, &after).await;
}

#[tokio::test]
async fn retiring_control_while_the_inner_queue_is_full_prevents_every_unsent_input() {
    let mut harness = Harness::start().await;
    harness.send_hello().await;
    let blocked = receive(&mut harness.peer).await;
    let mut requests = Vec::new();
    for _ in 0..QUEUE_CAPACITY + QUEUE_OVERFLOW {
        requests.push(harness.send_input(INPUT_PREFIX.to_vec()).await);
    }
    harness.dispatch_barrier().await;
    harness.scope.retire();
    finish_hello(&mut harness, &blocked).await;
    for expected in requests {
        let ControlEvent::Finished {
            request,
            result: Err(ControlError::Invalidated),
            connected: true,
        } = harness.event().await
        else {
            panic!()
        };
        assert_eq!(request.command, expected.command);
    }
    // A real subsequent request, rather than a timeout, proves that no retired
    // Input reached the transport ahead of this barrier.
    harness.send_hello().await;
    let after = receive(&mut harness.peer).await;
    assert!(matches!(after.fields().command, Command::Hello {}));
    finish_hello(&mut harness, &after).await;
}
