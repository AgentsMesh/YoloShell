use super::*;
use crate::claude_hooks::{self, ClaudeNativeResponse};
use crate::delivery_test_support::*;
use uuid::Uuid;

#[tokio::test]
async fn missing_listener_succeeds_only_after_durable_enqueue() {
    let fixture = Fixture::new();
    let event = fixture.start();
    deliver_with(&fixture.client, event.clone(), SHORT_BUDGET)
        .await
        .unwrap();
    let retained = fixture.next().unwrap();
    assert_eq!(retained.event, event);
    assert_eq!(retained.scope, fixture.client.scope);
    assert!(fixture.client.outbox.is_file());
}

#[tokio::test]
async fn persistence_failure_is_not_reported_as_successful_delivery() {
    let mut fixture = Fixture::new();
    fixture.client.outbox = fixture.home.path().to_path_buf();
    assert!(
        deliver_with(&fixture.client, fixture.start(), SHORT_BUDGET)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn exact_ack_drains_the_durable_queue() {
    let fixture = Fixture::new();
    let listener = fixture.listener();
    let event = fixture.start();
    let expected = event.clone();
    let server = tokio::spawn(async move {
        let (mut stream, request) = receive(&listener).await;
        assert_eq!(request.event, expected);
        respond(&mut stream, &accepted(&request)).await;
    });
    deliver_with(&fixture.client, event, DELIVERY_BUDGET)
        .await
        .unwrap();
    server.await.unwrap();
    assert!(fixture.next().is_none());
}

#[tokio::test]
async fn every_mismatched_ack_retries_the_identical_request() {
    let mutations: &[fn(&mut ClaudeNativeResponse)] = &[
        |response| response.schema_version = WRONG_SCHEMA,
        |response| response.scope.runtime_epoch = Uuid::new_v4(),
        |response| response.scope.terminal_id = Uuid::new_v4(),
        |response| response.event_sequence = OTHER_SEQUENCE,
        |response| response.event_id = Uuid::new_v4(),
        |response| response.accepted = false,
        |response| response.code = WRONG_CODE.into(),
        |response| response.invocation_id = Some(Uuid::nil()),
        |response| response.revision = None,
    ];
    for mutate in mutations {
        let fixture = Fixture::new();
        let listener = fixture.listener();
        let mutate = *mutate;
        let server = tokio::spawn(async move {
            let (mut first, request) = receive(&listener).await;
            let mut wrong = accepted(&request);
            mutate(&mut wrong);
            respond(&mut first, &wrong).await;
            let (mut second, retried) = receive(&listener).await;
            assert_eq!(retried, request);
            respond(&mut second, &accepted(&retried)).await;
        });
        deliver_with(&fixture.client, fixture.start(), DELIVERY_BUDGET)
            .await
            .unwrap();
        server.await.unwrap();
        assert!(fixture.next().is_none());
    }
}

#[tokio::test]
async fn transient_rejection_revalidates_the_same_event_before_acknowledging() {
    let fixture = Fixture::new();
    let listener = fixture.listener();
    let event = fixture.start();
    let server = tokio::spawn(async move {
        let (mut stream, request) = receive(&listener).await;
        let mut rejected = accepted(&request);
        rejected.accepted = false;
        rejected.code = claude_hooks::CLAUDE_HOOK_REJECTED.into();
        rejected.invocation_id = None;
        rejected.revision = None;
        respond(&mut stream, &rejected).await;
        let (mut retry, retried) = receive(&listener).await;
        assert_eq!(retried, request);
        respond(&mut retry, &accepted(&retried)).await;
    });
    deliver_with(&fixture.client, event.clone(), DELIVERY_BUDGET)
        .await
        .unwrap();
    server.await.unwrap();
    assert!(fixture.next().is_none());
}

#[tokio::test]
async fn lost_ack_replays_same_event_before_any_later_event() {
    let fixture = Fixture::new();
    let start = fixture.start();
    claude_hooks::native_outbox::enqueue(
        &fixture.client.outbox,
        fixture.client.scope,
        start.clone(),
    )
    .unwrap();
    let listener = fixture.listener();
    let running = fixture.running();
    let expected = running.clone();
    let server = tokio::spawn(async move {
        let (lost, request) = receive(&listener).await;
        assert_eq!(request.event, start);
        drop(lost);
        let (mut replay, retried) = receive(&listener).await;
        assert_eq!(retried, request);
        respond(&mut replay, &accepted(&retried)).await;
        let (mut next, subsequent) = receive(&listener).await;
        assert_eq!(subsequent.event, expected);
        assert_eq!(subsequent.event_sequence, request.event_sequence + REVISION);
        respond(&mut next, &accepted(&subsequent)).await;
    });
    deliver_with(&fixture.client, running, DELIVERY_BUDGET)
        .await
        .unwrap();
    server.await.unwrap();
    assert!(fixture.next().is_none());
}

#[tokio::test]
async fn client_rejects_foreign_scope_before_opening_socket() {
    let fixture = Fixture::new();
    claude_hooks::native_outbox::enqueue(
        &fixture.client.outbox,
        fixture.client.scope,
        fixture.start(),
    )
    .unwrap();
    let mut request = fixture.next().unwrap();
    request.scope.runtime_epoch = Uuid::new_v4();
    assert!(fixture.client.send(&request).await.is_err());
}
