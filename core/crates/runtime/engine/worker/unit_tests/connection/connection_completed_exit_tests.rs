use crate::worker::tests::*;
const COMPLETED_EXIT_AUTH_REQUEST_ID: &str = "completed-exit-auth";
const COMPLETED_EXIT_ATTACH_REQUEST_ID: &str = "completed-exit-attach";
pub(super) const COMPLETED_EXIT_CODE: i32 = 0;
const COMPLETED_EXIT_TIMESTAMP: i64 = 1;
const COMPLETED_EXIT_OUTPUT_SEQUENCE: u64 = 1;
const COMPLETED_EXIT_AFTER_SEQUENCE: u64 = 0;

#[derive(Clone, Copy)]
enum CompletedExitInterleaving {
    Cached,
    Broadcast,
    CachedAndBroadcast,
}

#[tokio::test]
async fn attachment_replays_a_durable_exit_completed_before_subscription() {
    let fixture = Fixture::new();
    let exit = ExitRecord {
        runtime_epoch: fixture.state.runtime_epoch(),
        code: Some(COMPLETED_EXIT_CODE),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: COMPLETED_EXIT_TIMESTAMP,
        last_output_sequence: COMPLETED_EXIT_OUTPUT_SEQUENCE,
    };
    *lock_recover(&fixture.state.exit) = Some(exit.clone());
    let (mut client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });

    let auth = RequestEnvelope::new(
        COMPLETED_EXIT_AUTH_REQUEST_ID,
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: FIXTURE_AUTH_TOKEN.into(),
            controller_generation: FIXTURE_CONTROLLER_GENERATION,
        })
        .unwrap(),
    );
    write_frame(&mut client, &auth).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    assert!(response.ok);
    let mut client = WorkerConnection::open(client).await.unwrap();

    let attach = RequestEnvelope::new(
        COMPLETED_EXIT_ATTACH_REQUEST_ID,
        method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id: fixture.terminal_id(),
            after_seq: COMPLETED_EXIT_AFTER_SEQUENCE,
        })
        .unwrap(),
    );
    let response = client.exchange(&attach).await.unwrap();
    assert!(response.ok);
    let event = client.next_event().await.unwrap();
    assert_completed_attachment_event(event, &fixture, &exit);
    assert!(client.next_event().await.is_err());
    drop(client);
    let _ = task.await.unwrap();
}

#[tokio::test]
async fn attachment_replays_one_exit_cached_after_subscription() {
    assert_completed_exit_interleaving(CompletedExitInterleaving::Cached).await;
}

#[tokio::test]
async fn attachment_forwards_one_exit_broadcast_after_subscription() {
    assert_completed_exit_interleaving(CompletedExitInterleaving::Broadcast).await;
}

#[tokio::test]
async fn attachment_forwards_one_exit_when_cache_and_broadcast_are_both_visible() {
    assert_completed_exit_interleaving(CompletedExitInterleaving::CachedAndBroadcast).await;
}

async fn assert_completed_exit_interleaving(interleaving: CompletedExitInterleaving) {
    let fixture = Fixture::new();
    let exit = completed_attachment_exit(&fixture);
    fixture
        .state
        .control_test_hooks
        .attachment_after_subscribe
        .arm();
    tokio::time::timeout(CONTROL_BARRIER_TIMEOUT, async {
        let (mut client, server) = UnixStream::pair().unwrap();
        let state = Arc::clone(&fixture.state);
        let task = tokio::spawn(async move { handle_connection(server, state).await });
        authenticate_completed_exit_client(&mut client).await;
        let client = WorkerConnection::open(client).await.unwrap();
        let pending = begin_worker_request(client, completed_exit_attach(fixture.terminal_id()));
        fixture
            .state
            .control_test_hooks
            .attachment_after_subscribe
            .wait_until_reached()
            .await;
        if matches!(
            interleaving,
            CompletedExitInterleaving::Cached | CompletedExitInterleaving::CachedAndBroadcast
        ) {
            *lock_recover(&fixture.state.exit) = Some(exit.clone());
        }
        if matches!(
            interleaving,
            CompletedExitInterleaving::Broadcast | CompletedExitInterleaving::CachedAndBroadcast
        ) {
            fixture
                .state
                .events
                .send(WorkerEvent::Exited(exit.clone()))
                .unwrap();
        }
        fixture
            .state
            .control_test_hooks
            .attachment_after_subscribe
            .resume();
        let (mut client, response) = pending.await.unwrap();
        let response = response.unwrap();
        assert!(response.ok);
        let event = client.next_event().await.unwrap();
        assert_completed_attachment_event(event, &fixture, &exit);
        assert!(client.next_event().await.is_err());
        drop(client);
        let _ = task.await.unwrap();
    })
    .await
    .unwrap();
}

fn completed_attachment_exit(fixture: &Fixture) -> ExitRecord {
    ExitRecord {
        runtime_epoch: fixture.state.runtime_epoch(),
        code: Some(COMPLETED_EXIT_CODE),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: COMPLETED_EXIT_TIMESTAMP,
        last_output_sequence: COMPLETED_EXIT_OUTPUT_SEQUENCE,
    }
}

fn assert_completed_attachment_exit(actual: &ExitRecord, expected: &ExitRecord) {
    assert_eq!(actual.runtime_epoch, expected.runtime_epoch);
    assert_eq!(actual.code, expected.code);
    assert_eq!(actual.signal, expected.signal);
    assert_eq!(actual.reason, expected.reason);
    assert_eq!(actual.observed_at_ms, expected.observed_at_ms);
    assert_eq!(actual.last_output_sequence, expected.last_output_sequence);
}

fn assert_completed_attachment_event(
    event: EventEnvelope,
    fixture: &Fixture,
    expected: &ExitRecord,
) {
    assert_eq!(event.event, event::SESSION_EXITED);
    assert_eq!(event.terminal_id, Some(fixture.terminal_id().to_string()));
    assert_eq!(
        event.runtime_epoch,
        Some(fixture.state.runtime_epoch().to_string())
    );
    let reported: ExitRecord = from_value(event.payload).unwrap();
    assert_completed_attachment_exit(&reported, expected);
}

async fn authenticate_completed_exit_client(client: &mut UnixStream) {
    let auth = RequestEnvelope::new(
        COMPLETED_EXIT_AUTH_REQUEST_ID,
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: FIXTURE_AUTH_TOKEN.into(),
            controller_generation: FIXTURE_CONTROLLER_GENERATION,
        })
        .unwrap(),
    );
    write_frame(client, &auth).await.unwrap();
    let response: ResponseEnvelope = read_frame(client).await.unwrap();
    assert!(response.ok);
}

fn completed_exit_attach(terminal_id: Uuid) -> RequestEnvelope {
    RequestEnvelope::new(
        COMPLETED_EXIT_ATTACH_REQUEST_ID,
        method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id,
            after_seq: COMPLETED_EXIT_AFTER_SEQUENCE,
        })
        .unwrap(),
    )
}
