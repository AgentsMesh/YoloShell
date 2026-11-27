use crate::{harness::Harness, operations, peer::*, queries, query_requests::*, scoped_requests};
use yolo_hub_client_channels::{ControlError, ControlEvent, RequestScope};

#[tokio::test]
async fn disconnected_stale_and_retired_queries_are_rejected_before_any_query_socket_opens() {
    let mut harness = Harness::start();
    let (mut control, opening) = operations::awaiting_handshake(&harness).await;
    let disconnected = query();
    harness.send(None, disconnected.clone()).await;
    let ControlEvent::Finished {
        request,
        result: Err(ControlError::NotConnected),
        connected: false,
    } = harness.event().await
    else {
        panic!()
    };
    assert_eq!(request.command, disconnected);
    let hello = hello_reply(harness.endpoint.hub);
    reply(&mut control, &opening, hello).await;
    let generation = harness.ready().await;
    let stale = query();
    let old = uuid::Uuid::new_v4();
    harness.send(Some(old), stale.clone()).await;
    assert!(matches!(
        queries::failure(&mut harness, old, &stale, true).await.1,
        ControlError::StaleGeneration
    ));
    let scope = RequestScope::default();
    let validity = scope.validity_until(deadline().into_std());
    scope.retire();
    let retired = query();
    scoped_requests::send(&harness, generation, retired.clone(), validity).await;
    assert!(matches!(
        queries::failure(&mut harness, generation, &retired, true)
            .await
            .1,
        ControlError::Invalidated
    ));
    // An incorrectly opened rejected request would be accepted before this one.
    let (mut current, _) = queries::dispatched(&harness, generation).await;
    harness.finish().await;
    eof(&mut control).await;
    eof(&mut current).await;
}

#[tokio::test]
async fn query_scope_retirement_during_handshake_prevents_dispatch() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let scope = RequestScope::default();
    let command = query();
    scoped_requests::send(
        &harness,
        generation,
        command.clone(),
        scope.validity_until(deadline().into_std()),
    )
    .await;
    let (mut query_peer, opening) = operations::awaiting_handshake(&harness).await;
    scope.retire();
    let hello = hello_reply(harness.endpoint.hub);
    reply(&mut query_peer, &opening, hello).await;
    eof(&mut query_peer).await;
    assert!(matches!(
        queries::failure(&mut harness, generation, &command, true)
            .await
            .1,
        ControlError::Invalidated
    ));
    harness.finish().await;
    eof(&mut control).await;
}

#[tokio::test]
async fn fast_disconnect_during_query_handshake_reports_its_original_request_without_reconnect() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let command = query();
    harness.send(Some(generation), command.clone()).await;
    let (mut query_peer, _) = operations::awaiting_handshake(&harness).await;
    operations::break_control(&harness, &mut control, generation).await;
    operations::control_failed(&mut harness, generation).await;
    let (_, error) = queries::failure(&mut harness, generation, &command, false).await;
    assert!(matches!(error, ControlError::StaleGeneration));
    eof(&mut query_peer).await;
    harness.finish().await;
}

#[tokio::test]
async fn dispatched_query_keeps_exact_result_after_scope_retirement_during_fast_reconnect() {
    let mut harness = Harness::start();
    let mut control = harness.accept().await;
    let generation = harness.ready().await;
    let scope = RequestScope::default();
    let command = query();
    let validity = scope.validity_until(deadline().into_std());
    scoped_requests::send(&harness, generation, command.clone(), validity.clone()).await;
    let mut query_peer = harness.accept().await;
    let request = receive(&mut query_peer).await;
    scope.retire();
    operations::break_control(&harness, &mut control, generation).await;
    operations::control_failed(&mut harness, generation).await;
    let (mut reconnecting, opening) = operations::awaiting_handshake(&harness).await;
    let result = ready(&command, harness.endpoint.hub);
    reply(&mut query_peer, &request, result.clone()).await;
    let ControlEvent::Finished {
        request,
        result: actual,
        connected: false,
    } = harness.event().await
    else {
        panic!("a query result must not wait for a new fast handshake or change generation")
    };
    assert_eq!(request.command, command);
    assert_eq!(request.generation, Some(generation));
    assert!(request.validity.unwrap().same_scope(&validity));
    assert_eq!(actual.unwrap(), result);
    eof(&mut query_peer).await;
    let hello = hello_reply(harness.endpoint.hub);
    reply(&mut reconnecting, &opening, hello).await;
    let fresh = harness.ready().await;
    assert_ne!(fresh, generation);
    harness.finish().await;
    eof(&mut reconnecting).await;
}
