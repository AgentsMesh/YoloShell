use crate::{commands::*, harness::Harness, operations::*, peer::*};
use tokio::time::timeout_at;
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn blocked_creation_does_not_delay_input_renewal_release_or_catalogue() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let (mut operation, creation) = dispatched(&harness, generation).await;
    let grant = grant(harness.endpoint.hub);
    for (command, response) in [
        (input(grant), Reply::Input { duplicate: false }),
        (Command::Renew { grant }, Reply::Lease { grant }),
        (Command::Release { grant }, Reply::Completed {}),
    ] {
        round_trip(&mut harness, &mut control, generation, command, response).await;
    }
    let request_id = harness.refresh();
    let mut catalog = harness.accept().await;
    let request = receive(&mut catalog).await;
    assert!(matches!(request.fields().command, Command::List { .. }));
    reply(
        &mut catalog,
        &request,
        Reply::Terminals {
            terminals: vec![],
            next: None,
        },
    )
    .await;
    let event = timeout_at(deadline(), harness.catalogs.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.request_id, request_id);
    assert!(event.result.unwrap().terminals.is_empty());
    assert!(harness.controls.try_recv().is_err());
    let result = created(harness.endpoint.hub);
    reply(&mut operation, &creation, result.clone()).await;
    completed(&mut harness, generation, &creation.fields().command, result).await;
    harness.finish().await;
    eof(&mut control).await;
    eof(&mut catalog).await;
    eof(&mut operation).await;
}

#[tokio::test]
async fn lost_creation_reply_keeps_fast_control_connected_to_its_original_generation() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let (operation, creation) = dispatched(&harness, generation).await;
    drop(operation);
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::Client(error)),
        connected: true,
    } = harness.event().await
    else {
        panic!("losing a lifecycle reply must not disconnect a healthy writer channel")
    };
    assert_eq!(request.generation, Some(generation));
    assert_eq!(request.command, creation.fields().command);
    assert!(error.outcome_is_unknown());
    let grant = grant(harness.endpoint.hub);
    round_trip(
        &mut harness,
        &mut control,
        generation,
        Command::Renew { grant },
        Reply::Lease { grant },
    )
    .await;
    cancel_without_replay(&mut harness).await;
    eof(&mut control).await;
}

#[tokio::test]
async fn cancelling_the_owner_closes_control_catalogue_dispatched_and_opening_operations() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let (mut dispatched, _) = dispatched(&harness, generation).await;
    harness.send(Some(generation), create()).await;
    let (mut opening, _) = awaiting_handshake(&harness).await;
    harness.refresh();
    let mut catalog = harness.accept().await;
    receive(&mut catalog).await;
    cancel_without_replay(&mut harness).await;
    eof(&mut control).await;
    eof(&mut catalog).await;
    eof(&mut dispatched).await;
    // The pending operation must close after Hello, without sending Create.
    eof(&mut opening).await;
}
