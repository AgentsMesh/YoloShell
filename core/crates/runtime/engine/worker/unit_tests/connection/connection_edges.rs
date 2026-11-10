use crate::worker::tests::*;
const PEER_EOF_TIMEOUT: Duration = Duration::from_secs(1);
const CONNECTION_EDGE_OUTPUT_SEQUENCE: u64 = 1;
const CONNECTION_EDGE_CONTROLLER_GENERATION: u64 = 1;
const CONNECTION_EDGE_REPLACEMENT_GENERATION: u64 = CONNECTION_EDGE_CONTROLLER_GENERATION + 1;
const CONNECTION_EDGE_OVERFLOW_INCREMENT: u64 = 1;
const CONNECTION_EDGE_EMPTY_BARRIER: u64 = 0;
const CONNECTION_EDGE_CLOSED_CHANNEL_CAPACITY: usize = 1;
const CONNECTION_EDGE_OUTPUT_BYTE: u8 = b'x';
const CONNECTION_EDGE_AUTH_METHOD_REQUEST_ID: &str = "auth-method-request";
const CONNECTION_EDGE_INVALID_AUTH_METHOD: &str = "wrong-auth-method";
const CONNECTION_EDGE_AUTH_TOKEN_REQUEST_ID: &str = "token";
const CONNECTION_EDGE_INVALID_AUTH_TOKEN: &str = "wrong";
const CONNECTION_EDGE_STATUS_REQUEST_ID: &str = "status-request";
const CONNECTION_EDGE_CONFLICT_CODE: &str = "conflict";
const CONNECTION_EDGE_IDLE_CLOSE_DETAIL: &str = "idle attachment must observe peer EOF";

pub(super) async fn raw_worker_connection(
    fixture: &Fixture,
) -> (UnixStream, tokio::task::JoinHandle<Result<()>>) {
    let (client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });
    (client, task)
}

#[tokio::test]
async fn worker_auth_rejects_missing_method_and_wrong_token() {
    let fixture = Fixture::new();
    let (mut client, task) = raw_worker_connection(&fixture).await;
    let request = RequestEnvelope::new(
        CONNECTION_EDGE_AUTH_METHOD_REQUEST_ID,
        CONNECTION_EDGE_INVALID_AUTH_METHOD,
        yolo_capnp_value::Value::Null,
    );
    write_frame(&mut client, &request).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::UNAUTHORIZED
    );
    task.await.unwrap().unwrap();

    let (mut client, task) = raw_worker_connection(&fixture).await;
    let request = RequestEnvelope::new(
        CONNECTION_EDGE_AUTH_TOKEN_REQUEST_ID,
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: CONNECTION_EDGE_INVALID_AUTH_TOKEN.into(),
            controller_generation: CONNECTION_EDGE_CONTROLLER_GENERATION,
        })
        .unwrap(),
    );
    write_frame(&mut client, &request).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::UNAUTHORIZED
    );
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn authenticated_connection_observes_later_controller_fencing() {
    let fixture = Fixture::new();
    let (mut client, task) =
        authenticated_worker_connection(&fixture, CONNECTION_EDGE_CONTROLLER_GENERATION).await;
    fixture
        .state
        .controller_generation
        .store(CONNECTION_EDGE_REPLACEMENT_GENERATION, Ordering::Release);
    let request = RequestEnvelope::new(
        CONNECTION_EDGE_STATUS_REQUEST_ID,
        WORKER_STATUS_METHOD,
        yolo_capnp_value::Value::Null,
    );
    let response = client.exchange(&request).await.unwrap();
    assert_eq!(response.error.unwrap().code, CONNECTION_EDGE_CONFLICT_CODE);
    drop(client);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn attachment_reports_broadcast_lag_as_an_output_gap() {
    let fixture = Fixture::new();
    let (mut client, task) =
        authenticated_worker_connection(&fixture, CONNECTION_EDGE_CONTROLLER_GENERATION).await;
    attach_worker(&mut client, &fixture).await;
    for sequence in CONNECTION_EDGE_OUTPUT_SEQUENCE
        ..=(WORKER_EVENT_CHANNEL_CAPACITY as u64 + CONNECTION_EDGE_OVERFLOW_INCREMENT)
    {
        fixture
            .state
            .events
            .send(WorkerEvent::Output(OutputChunk {
                runtime_epoch: fixture.state.runtime_epoch(),
                sequence,
                data: vec![CONNECTION_EDGE_OUTPUT_BYTE],
            }))
            .unwrap();
    }
    let event = client.next_event().await.unwrap();
    assert_eq!(event.event, event::OUTPUT_GAP);
    let payload: OutputGapPayload = from_value(event.payload).unwrap();
    assert!(payload.skipped > 0);
    assert!(payload.resync_required);
    drop(client);
    let _ = task.await.unwrap();
}

#[tokio::test]
async fn parser_screen_reset_is_forwarded_as_an_output_gap() {
    let fixture = Fixture::new();
    let (mut client, task) =
        authenticated_worker_connection(&fixture, CONNECTION_EDGE_CONTROLLER_GENERATION).await;
    attach_worker(&mut client, &fixture).await;
    fixture
        .state
        .events
        .send(WorkerEvent::ScreenReset(OutputGapPayload {
            skipped: SCREEN_RESET_SKIPPED_CHUNKS,
            resync_required: true,
        }))
        .unwrap();
    let event = client.next_event().await.unwrap();
    assert_eq!(event.event, event::OUTPUT_GAP);
    let payload: OutputGapPayload = from_value(event.payload).unwrap();
    assert_eq!(payload.skipped, SCREEN_RESET_SKIPPED_CHUNKS);
    assert!(payload.resync_required);
    drop(client);
    let _ = task.await.unwrap();
}

#[tokio::test]
async fn idle_attachment_stops_cleanly_after_peer_disconnect() {
    let fixture = Fixture::new();
    let (mut client, task) =
        authenticated_worker_connection(&fixture, CONNECTION_EDGE_CONTROLLER_GENERATION).await;
    attach_worker(&mut client, &fixture).await;
    drop(client);
    tokio::time::timeout(PEER_EOF_TIMEOUT, task)
        .await
        .expect(CONNECTION_EDGE_IDLE_CLOSE_DETAIL)
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn barrier_receiver_surfaces_closed_channels() {
    let (sender, mut receiver) = broadcast::channel(CONNECTION_EDGE_CLOSED_CHANNEL_CAPACITY);
    drop(sender);
    assert!(matches!(
        recv_after_barrier(&mut receiver, CONNECTION_EDGE_EMPTY_BARRIER).await,
        Err(broadcast::error::RecvError::Closed)
    ));
}

#[tokio::test(start_paused = true)]
async fn unresponsive_worker_authentication_observes_socket_deadline() {
    let fixture = Fixture::new();
    let (client, task) = raw_worker_connection(&fixture).await;

    let error = task.await.unwrap().unwrap_err();
    assert!(error.to_string().contains(WORKER_OPERATION_TIMEOUT_CONTEXT));
    drop(client);
}

include!("connection_controller_fencing.rs");
