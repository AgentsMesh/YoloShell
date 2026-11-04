use crate::{commands::grant, harness::Harness, ordered_inputs::*, peer::*, scoped_requests::send};
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_client_channels::{ControlError, ControlEvent, RequestScope};
use yolo_hub_rpc_protocol::Reply;

#[tokio::test]
async fn an_expired_prefix_cancels_the_newer_suffix_before_any_completion_is_consumed() {
    let mut harness = Harness::start();
    let mut peer = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let scope = RequestScope::ordered_inputs();
    let prefix = input(grant, PREFIX);
    let suffix = input(grant, SUFFIX);
    send(
        &harness,
        generation,
        prefix.clone(),
        scope.validity_until(Instant::now()),
    )
    .await;
    send(&harness, generation, suffix.clone(), validity(&scope)).await;
    wire_barrier(&harness, &mut peer, generation).await;
    rejected(&mut harness, &prefix).await;
    rejected(&mut harness, &suffix).await;
    completed_barrier(&mut harness).await;
    harness.finish().await;
    eof(&mut peer).await;
}

#[tokio::test]
async fn stale_generation_rejection_cancels_dependents_even_if_their_generation_is_current() {
    let mut harness = Harness::start();
    let mut peer = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let scope = RequestScope::ordered_inputs();
    let prefix = input(grant, PREFIX);
    let suffix = input(grant, SUFFIX);
    send(&harness, Uuid::new_v4(), prefix.clone(), validity(&scope)).await;
    send(&harness, generation, suffix.clone(), validity(&scope)).await;
    wire_barrier(&harness, &mut peer, generation).await;
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::StaleGeneration),
        connected: true,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(request.command, prefix);
    rejected(&mut harness, &suffix).await;
    completed_barrier(&mut harness).await;
    harness.finish().await;
    eof(&mut peer).await;
}

#[tokio::test]
async fn ordinary_scopes_keep_independent_input_deadlines_after_an_expired_request() {
    let mut harness = Harness::start();
    let mut peer = harness.accept().await;
    let generation = harness.ready().await;
    let grant = grant(harness.endpoint.hub);
    let scope = RequestScope::default();
    let prefix = input(grant, PREFIX);
    let independent = input(grant, SUFFIX);
    send(
        &harness,
        generation,
        prefix.clone(),
        scope.validity_until(Instant::now()),
    )
    .await;
    send(&harness, generation, independent.clone(), validity(&scope)).await;
    let request = receive(&mut peer).await;
    assert_eq!(request.fields().command, independent);
    reply(&mut peer, &request, Reply::Input { duplicate: false }).await;
    rejected(&mut harness, &prefix).await;
    crate::scoped_requests::delivered(&mut harness, &independent).await;
    assert!(validity(&scope).is_current());
    harness.finish().await;
    eof(&mut peer).await;
}
