use crate::{
    commands::grant, constants::TIMEOUT, harness::Harness, operations::*, peer::*,
    scoped_requests::send,
};
use std::time::{Duration, Instant};
use uuid::Uuid;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlEvent, RequestScope};
use yolo_hub_jsonrpc_transport::TransportError;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

const EXPIRY_MARGIN: Duration = Duration::from_secs(1);

#[tokio::test]
async fn retiring_scope_after_creation_dispatch_preserves_its_exact_known_result() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let scope = RequestScope::default();
    let command = create();
    send(
        &harness,
        generation,
        command.clone(),
        scope.validity_until(Instant::now() + TIMEOUT),
    )
    .await;
    let mut operation = harness.accept().await;
    let request = receive(&mut operation).await;
    assert_eq!(request.fields().command, command);
    scope.retire();
    let result = created(harness.endpoint.hub);
    reply(&mut operation, &request, result.clone()).await;
    completed(&mut harness, generation, &command, result).await;
    eof(&mut operation).await;
    renew_and_stop(&mut harness, &mut control, generation).await;
}

#[tokio::test]
async fn retiring_scope_during_handshake_sends_no_creation_and_preserves_fast_control() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let scope = RequestScope::default();
    let command = create();
    send(
        &harness,
        generation,
        command.clone(),
        scope.validity_until(Instant::now() + TIMEOUT),
    )
    .await;
    let (mut operation, handshake) = awaiting_handshake(&harness).await;
    scope.retire();
    reply(
        &mut operation,
        &handshake,
        Reply::Hello {
            hello: Hello {
                hub_id: harness.endpoint.hub,
                version: HUB_PROTOCOL_VERSION,
                methods: vec![Method::Hello, Method::Create],
            },
        },
    )
    .await;
    // The next frame must be EOF, so an invalidated Create cannot escape after
    // the independently authenticated lifecycle connection completes Hello.
    eof(&mut operation).await;
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::Invalidated),
        connected: true,
    } = harness.event().await
    else {
        panic!("retired scope must be rejected before dispatch without disconnecting control")
    };
    assert_eq!(request.generation, Some(generation));
    assert_eq!(request.command, command);
    renew_and_stop(&mut harness, &mut control, generation).await;
}

#[tokio::test]
async fn creation_validity_deadline_bounds_its_wait_without_replay_or_fast_control_loss() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let scope = RequestScope::default();
    let expires = Instant::now() + TIMEOUT;
    let command = create();
    send(
        &harness,
        generation,
        command.clone(),
        scope.validity_until(expires),
    )
    .await;
    let mut operation = harness.accept().await;
    let dispatched = receive(&mut operation).await;
    assert_eq!(dispatched.fields().command, command);
    // Advance only after dispatch. The absolute local validity deadline is
    // shorter than the shell startup budget; no wall-clock sleep is needed.
    tokio::time::pause();
    tokio::time::advance(TIMEOUT + EXPIRY_MARGIN).await;
    let event = harness.event().await;
    tokio::time::resume();
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::Client(ClientError::OutcomeUnknown(TransportError::Deadline))),
        connected: true,
    } = event
    else {
        panic!("dispatched creation must retain uncertainty when its validity deadline expires")
    };
    assert_eq!(request.generation, Some(generation));
    assert_eq!(request.command, command);
    assert_eq!(request.validity.as_ref().unwrap().deadline(), expires);
    eof(&mut operation).await;
    renew_and_stop(&mut harness, &mut control, generation).await;
}

async fn renew_and_stop(harness: &mut Harness, control: &mut Peer, generation: Uuid) {
    let grant = grant(harness.endpoint.hub);
    round_trip(
        harness,
        control,
        generation,
        Command::Renew { grant },
        Reply::Lease { grant },
    )
    .await;
    cancel_without_replay(harness).await;
    eof(control).await;
}
