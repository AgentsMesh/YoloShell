use crate::{
    commands::grant, constants::TIMEOUT, harness::Harness, ordered_inputs::*, peer::*,
    scoped_requests::send,
};
use std::time::Instant;
use yolo_hub_client_channels::{ControlError, ControlEvent, InputStream};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn unknown_old_view_blocks_new_view_suffix_before_any_ui_completion_is_consumed() {
    let mut harness = Harness::start();
    let mut peer = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let stream = InputStream::new(Instant::now() + TIMEOUT);
    let previous = stream.view();
    let prefix = input(grant, PREFIX);
    send(&harness, generation, prefix.clone(), validity(&previous)).await;
    let dispatched = receive(&mut peer).await;
    assert_eq!(dispatched.fields().command, prefix);
    drop(previous);
    let current = stream.view();
    let suffix = input(grant, SUFFIX);
    send(&harness, generation, suffix.clone(), validity(&current)).await;
    reply(
        &mut peer,
        &dispatched,
        Reply::Error {
            code: PublicError::OutcomeUnknown,
        },
    )
    .await;

    // The next real wire request is independent Hello, before polling UI events.
    wire_barrier(&harness, &mut peer, generation).await;

    assert!(!stream.is_current());
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Err(ControlError::Client(_)),
            connected: true,
            ..
        }
    ));
    rejected(&mut harness, &suffix).await;
    completed_barrier(&mut harness).await;
    harness.finish().await;
    eof(&mut peer).await;
}

#[tokio::test]
async fn cancelled_undispatched_view_preserves_new_view_and_independent_control() {
    let mut harness = Harness::start();
    let mut peer = harness.accept().await;
    let generation = harness.ready().await;
    let stream = InputStream::new(Instant::now() + TIMEOUT);
    let previous = stream.view();
    let current = stream.view();
    let grant = grant(harness.endpoint.hub);
    let prefix = input(grant, PREFIX);
    let suffix = input(grant, SUFFIX);
    harness.send(Some(generation), Command::Hello {}).await;
    let gate = receive(&mut peer).await;
    send(&harness, generation, prefix.clone(), validity(&previous)).await;
    drop(previous);
    send(&harness, generation, suffix.clone(), validity(&current)).await;
    reply(&mut peer, &gate, hello_reply(harness.endpoint.hub)).await;
    let next = receive(&mut peer).await;
    assert_eq!(next.fields().command, suffix);
    reply(&mut peer, &next, Reply::Input { duplicate: false }).await;
    wire_barrier(&harness, &mut peer, generation).await;

    completed_barrier(&mut harness).await;
    rejected(&mut harness, &prefix).await;
    assert!(matches!(
        harness.event().await,
        ControlEvent::Finished {
            result: Ok(Reply::Input { duplicate: false }),
            ..
        }
    ));
    completed_barrier(&mut harness).await;
    assert!(stream.is_current());
    harness.finish().await;
    eof(&mut peer).await;
}
