use super::*;
use crate::delivery_test_support::*;

#[test]
fn native_delivery_uses_the_provider_management_handshake_contract() {
    assert_eq!(
        HOOK_DELIVERY_BUDGET,
        crate::claude_hooks::CLAUDE_MANAGEMENT_HANDSHAKE_TIMEOUT
    );
}

#[tokio::test]
async fn stalled_peer_is_bounded_and_keeps_durable_head() {
    let fixture = Fixture::new();
    let listener = fixture.listener();
    let (finish, finished) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, request) = receive(&listener).await;
        finished.await.unwrap();
        assert!(
            tokio::time::timeout(OBSERVATION_WINDOW, listener.accept())
                .await
                .is_err()
        );
        drop(stream);
        request
    });
    let event = fixture.start();
    tokio::time::timeout(
        COMPLETION_BUDGET,
        deliver_with(&fixture.client, event.clone(), SHORT_BUDGET),
    )
    .await
    .unwrap()
    .unwrap();
    finish.send(()).unwrap();
    let observed = server.await.unwrap();
    assert_eq!(fixture.next(), Some(observed));
    assert_eq!(fixture.next().unwrap().event, event);
}

#[tokio::test]
async fn next_callback_replays_queue_left_by_a_stalled_peer() {
    let fixture = Fixture::new();
    let listener = fixture.listener();
    let event = fixture.start();
    let (finish, finished) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, first) = receive(&listener).await;
        finished.await.unwrap();
        drop(stream);
        let (mut replay, retried) = receive(&listener).await;
        assert_eq!(retried, first);
        respond(&mut replay, &accepted(&retried)).await;
        let (mut next, following) = receive(&listener).await;
        assert!(following.event_sequence > retried.event_sequence);
        respond(&mut next, &accepted(&following)).await;
    });
    deliver_with(&fixture.client, event.clone(), SHORT_BUDGET)
        .await
        .unwrap();
    assert_eq!(fixture.next().unwrap().event, event);
    finish.send(()).unwrap();
    deliver_with(&fixture.client, fixture.running(), DELIVERY_BUDGET)
        .await
        .unwrap();
    server.await.unwrap();
    assert!(fixture.next().is_none());
}

#[tokio::test]
async fn zero_budget_durably_enqueues_without_starting_transport() {
    let fixture = Fixture::new();
    let listener = fixture.listener();
    let event = fixture.start();
    deliver_with(&fixture.client, event.clone(), std::time::Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(fixture.next().unwrap().event, event);
    assert!(
        tokio::time::timeout(OBSERVATION_WINDOW, listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn persistent_rejection_is_bounded_and_never_discards_the_durable_event() {
    let fixture = Fixture::new();
    let listener = fixture.listener();
    let (finish, mut finished) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let mut expected = None;
        loop {
            let (mut stream, request) = tokio::select! {
                _ = &mut finished => break,
                received = receive(&listener) => received,
            };
            if let Some(expected) = &expected {
                assert_eq!(expected, &request);
            }
            expected = Some(request.clone());
            let mut rejected = accepted(&request);
            rejected.accepted = false;
            rejected.code = crate::claude_hooks::CLAUDE_HOOK_REJECTED.into();
            rejected.invocation_id = None;
            rejected.revision = None;
            respond(&mut stream, &rejected).await;
        }
        expected.unwrap()
    });
    tokio::time::timeout(
        COMPLETION_BUDGET,
        deliver_with(&fixture.client, fixture.start(), SHORT_BUDGET),
    )
    .await
    .unwrap()
    .unwrap();
    finish.send(()).unwrap();
    assert_eq!(fixture.next(), Some(server.await.unwrap()));
}
