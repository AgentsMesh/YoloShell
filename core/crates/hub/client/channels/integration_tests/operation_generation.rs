use crate::{commands::grant, harness::Harness, operations::*, peer::*};
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn control_generation_lost_during_handshake_sends_no_creation_bytes() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let old = harness.ready().await;
    let command = create();
    harness.send(Some(old), command.clone()).await;
    let (mut operation, _) = awaiting_handshake(&harness).await;
    break_control(&harness, &mut control, old).await;
    control_failed(&mut harness, old).await;
    // Cancellation closes the operation immediately, even though completion
    // delivery waits until a fast channel is available for follow-up work.
    eof(&mut operation).await;
    let mut current = harness.accept().await;
    let fresh = harness.ready().await;
    assert_ne!(fresh, old);
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::StaleGeneration),
        connected: true,
    } = harness.event().await
    else {
        panic!("handshake must be cancelled when its admission generation is retired")
    };
    assert_eq!(request.generation, Some(old));
    assert_eq!(request.command, command);
    let grant = grant(harness.endpoint.hub);
    round_trip(
        &mut harness,
        &mut current,
        fresh,
        Command::Renew { grant },
        Reply::Lease { grant },
    )
    .await;
    cancel_without_replay(&mut harness).await;
    eof(&mut current).await;
}

#[tokio::test]
async fn creation_completed_while_fast_control_is_offline_survives_until_the_next_ready() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let old = harness.ready().await;
    let (mut operation, request) = dispatched(&harness, old).await;
    break_control(&harness, &mut control, old).await;
    control_failed(&mut harness, old).await;
    let result = created(harness.endpoint.hub);
    reply(&mut operation, &request, result.clone()).await;
    // The independent stream has finished before any replacement control
    // handshake. Retaining its bounded result must not retain the socket.
    eof(&mut operation).await;
    assert!(harness.controls.try_recv().is_err());
    let mut current = harness.accept().await;
    let fresh = harness.ready().await;
    assert_ne!(old, fresh);
    completed(&mut harness, old, &request.fields().command, result).await;
    let grant = grant(harness.endpoint.hub);
    round_trip(
        &mut harness,
        &mut current,
        fresh,
        Command::Renew { grant },
        Reply::Lease { grant },
    )
    .await;
    cancel_without_replay(&mut harness).await;
    eof(&mut current).await;
}

#[tokio::test]
async fn dispatched_creation_keeps_its_old_correlation_after_fast_control_reconnects() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let old = harness.ready().await;
    let (mut operation, request) = dispatched(&harness, old).await;
    break_control(&harness, &mut control, old).await;
    control_failed(&mut harness, old).await;
    let mut current = harness.accept().await;
    let fresh = harness.ready().await;
    assert_ne!(old, fresh);
    let result = created(harness.endpoint.hub);
    reply(&mut operation, &request, result.clone()).await;
    // Late completion retains the old request generation while connected=true
    // describes the current fast channel; it is never a new Ready event.
    completed(&mut harness, old, &request.fields().command, result).await;
    eof(&mut operation).await;
    harness.send(Some(old), Command::Hello {}).await;
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::StaleGeneration),
        connected: true,
    } = harness.event().await
    else {
        panic!("a late completion must not restore the old control generation")
    };
    assert_eq!(request.generation, Some(old));
    assert!(matches!(request.command, Command::Hello {}));
    let grant = grant(harness.endpoint.hub);
    round_trip(
        &mut harness,
        &mut current,
        fresh,
        Command::Renew { grant },
        Reply::Lease { grant },
    )
    .await;
    cancel_without_replay(&mut harness).await;
    eof(&mut current).await;
}
