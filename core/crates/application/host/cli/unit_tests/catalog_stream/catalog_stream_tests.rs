use super::*;
use crate::catalog_stream::catalog_stream_framing::{
    FRAME_LENGTH_BYTES, FrameError, MAXIMUM_FRAME_BYTES,
};
use tokio::net::UnixListener;
use tokio::time::Duration;
use yolo_client::{Client, ClientError};
use yolo_runtime::cli::constants as cli;
use yolo_runtime::host::{ListSessionsRequest, ListSessionsResult};
use yolo_runtime::protocol::{RequestEnvelope, ResponseEnvelope};

const TEST_STREAM_BYTES: usize = 16_384;
const REQUEST_COUNT: usize = 3;
const LATE_RESPONSE_DELAY: Duration = Duration::from_millis(40);
const EARLY_TIMEOUT: Duration = Duration::from_millis(5);
const INCOMPLETE_SOCKET_NAME: &str = "incomplete.sock";

#[tokio::test]
async fn repeated_catalog_requests_reuse_one_rpc_connection() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("catalog.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
        for _ in 0..REQUEST_COUNT {
            respond_empty(&mut stream).await;
        }
    });
    let connection = CatalogConnection::existing(Client::new(socket));
    let (mut requests, input) = tokio::io::duplex(TEST_STREAM_BYTES);
    let (output, mut responses) = tokio::io::duplex(TEST_STREAM_BYTES);
    let serving = tokio::spawn(serve(connection, input, output));

    for _ in 0..REQUEST_COUNT {
        let request = CatalogRequest {
            id: uuid::Uuid::new_v4(),
            command: CatalogCommand::List,
            deadline_ms: cli::CATALOG_DEFAULT_DEADLINE_MILLISECONDS,
        };
        write_catalog_frame(&mut requests, &request).await.unwrap();
        let response: serde_json::Value = read_catalog_frame(&mut responses).await.unwrap();
        assert_eq!(response["id"], request.id.to_string());
        assert_eq!(response["ok"], true);
        assert_eq!(response["sessions"], serde_json::json!([]));
    }
    drop(requests);
    serving.await.unwrap().unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn disconnected_rpc_connection_reconnects_within_bounded_request() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("reconnect.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
            respond_empty(&mut stream).await;
        }
    });
    let mut connection = CatalogConnection::existing(Client::new(socket));
    assert!(connection.list_sessions().await.unwrap().is_empty());
    assert!(connection.list_sessions().await.unwrap().is_empty());
    server.await.unwrap();
}

#[tokio::test]
async fn incomplete_catalog_attestation_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(INCOMPLETE_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
        respond_catalog(&mut stream, false).await;
    });
    let mut connection = CatalogConnection::existing(Client::new(socket));

    let error = connection.list_sessions().await.unwrap_err();
    assert!(matches!(
        error.downcast_ref::<ClientError>(),
        Some(ClientError::IncompleteCatalogInventory)
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn oversized_and_malformed_frames_fail_closed_before_connecting() {
    let oversized = ((MAXIMUM_FRAME_BYTES + 1) as u32).to_be_bytes();
    let connection = CatalogConnection::existing(Client::new("/missing/catalog.sock"));
    let error = serve(connection, oversized.as_slice(), tokio::io::sink())
        .await
        .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<FrameError>(),
        Some(FrameError::TooLarge { length }) if *length == MAXIMUM_FRAME_BYTES + 1
    ));

    let payload = br#"{"id":"00000000-0000-0000-0000-000000000001","command":"list","extra":1}"#;
    let mut encoded = Vec::from((payload.len() as u32).to_be_bytes());
    encoded.extend_from_slice(payload);
    let decoded = read_catalog_frame::<_, CatalogRequest>(&mut encoded.as_slice()).await;
    assert!(matches!(decoded, Err(FrameError::Encode(_))));
    assert_eq!(FRAME_LENGTH_BYTES, std::mem::size_of::<u32>());
}

#[tokio::test]
async fn partial_frames_and_timed_out_late_responses_are_never_reused() {
    for partial in [vec![0_u8], vec![0_u8; FRAME_LENGTH_BYTES - 1]] {
        let decoded = read_catalog_frame::<_, CatalogRequest>(&mut partial.as_slice()).await;
        assert!(matches!(decoded, Err(FrameError::Io(_))));
    }
    let body_length = 2_u32.to_be_bytes();
    let mut partial_body = Vec::from(body_length);
    partial_body.push(b'{');
    let decoded = read_catalog_frame::<_, CatalogRequest>(&mut partial_body.as_slice()).await;
    assert!(matches!(decoded, Err(FrameError::Io(_))));

    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("late.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (stale, _) = listener.accept().await.unwrap();
        let mut stale = yolo_runtime_rpc_testing::Peer::new(stale);
        let _: RequestEnvelope = stale.request().await.unwrap();
        tokio::time::sleep(LATE_RESPONSE_DELAY).await;
        assert!(stale.request().await.is_err());
        let (current, _) = listener.accept().await.unwrap();
        let mut current = yolo_runtime_rpc_testing::Peer::new(current);
        respond_empty(&mut current).await;
    });
    let mut connection = CatalogConnection::existing(Client::new(socket));
    let timed_out = connection.list_sessions_with_timeout(EARLY_TIMEOUT).await;
    assert!(timed_out.is_err());
    assert!(connection.list_sessions().await.unwrap().is_empty());
    server.await.unwrap();
}

async fn respond_empty(stream: &mut yolo_runtime_rpc_testing::Peer) {
    respond_catalog(stream, true).await;
}

async fn respond_catalog(stream: &mut yolo_runtime_rpc_testing::Peer, complete: bool) {
    let request = stream.request().await.unwrap();
    let payload: ListSessionsRequest = request.decode_payload().unwrap();
    assert!(!payload.include_archived);
    assert!(payload.require_complete);
    let response = ResponseEnvelope::success(
        &request,
        &ListSessionsResult {
            sessions: Vec::new(),
            complete,
        },
    )
    .unwrap();
    stream.respond(&response).await.unwrap();
}
