use crate::worker::tests::*;
const ATTACHMENT_AHEAD_REQUEST_ID: &str = "attachment-ahead";
const ATTACHMENT_EXHAUSTED_FORCE_REQUEST_ID: &str = "attachment-exhausted-force";
const ATTACHMENT_EXHAUSTED_REPLAY_REQUEST_ID: &str = "attachment-exhausted-replay";

#[tokio::test]
async fn attachment_ahead_cursor_returns_a_non_retryable_failure() {
    let fixture = Fixture::new();
    assert_attachment_cursor_failure(
        &fixture,
        ATTACHMENT_AHEAD_REQUEST_ID,
        ATTACHMENT_SEQUENCE_INCREMENT,
        ATTACHMENT_CURSOR_AHEAD_CONTEXT,
    )
    .await;
}

#[tokio::test]
async fn exhausted_force_snapshot_returns_a_non_retryable_failure() {
    let fixture = Fixture::new();
    fixture
        .state
        .journal
        .lock()
        .unwrap()
        .set_last_sequence_for_test(u64::MAX);
    assert_attachment_cursor_failure(
        &fixture,
        ATTACHMENT_EXHAUSTED_FORCE_REQUEST_ID,
        FORCE_SNAPSHOT_AFTER_SEQUENCE,
        ATTACHMENT_CURSOR_EXHAUSTED_CONTEXT,
    )
    .await;
}

#[tokio::test]
async fn exhausted_replay_returns_a_non_retryable_failure() {
    let fixture = Fixture::new();
    fixture
        .state
        .journal
        .lock()
        .unwrap()
        .set_last_sequence_for_test(u64::MAX);
    assert_attachment_cursor_failure(
        &fixture,
        ATTACHMENT_EXHAUSTED_REPLAY_REQUEST_ID,
        u64::MAX - ATTACHMENT_SEQUENCE_INCREMENT,
        ATTACHMENT_CURSOR_EXHAUSTED_CONTEXT,
    )
    .await;
}

async fn assert_attachment_cursor_failure(
    fixture: &Fixture,
    request_id: &str,
    after_seq: u64,
    expected_detail: &str,
) {
    let (mut client, task) =
        authenticated_worker_connection(fixture, FIXTURE_CONTROLLER_GENERATION).await;
    let request = RequestEnvelope::new(
        request_id,
        method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id: fixture.terminal_id(),
            after_seq,
        })
        .unwrap(),
    );

    let response = client.exchange(&request).await.unwrap();
    let error = response.error.unwrap();
    assert_eq!(error.code, crate::protocol::error_code::INVALID_REQUEST);
    assert_eq!(error.message, expected_detail);
    assert!(!error.retryable);

    drop(client);
    task.await.unwrap().unwrap();
}
