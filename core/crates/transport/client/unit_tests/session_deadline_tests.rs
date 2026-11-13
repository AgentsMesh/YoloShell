use super::api_matrix_fixtures::{MatrixScenario, response, test_session_id};
use super::constants::DEFAULT_CLIENT_OPERATION_TIMEOUT;
use super::deadline::ClientDeadline;
use super::*;
use crate::model::{HostStatus, PROTOCOL_VERSION};
use crate::protocol::{RequestEnvelope, ResponseEnvelope, method};
use std::future::pending;
use std::path::Path;
use std::time::{Duration, Instant as WallInstant};
use tokio::net::UnixListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const DEADLINE_STAGE_COUNT: u32 = 2;
const STATUS_SOCKET_NAME: &str = "status-shared-deadline.sock";
const CUSTOM_STATUS_SOCKET_NAME: &str = "status-custom-deadline.sock";
const SNAPSHOT_SOCKET_NAME: &str = "snapshot-shared-deadline.sock";
const SIGNAL_CLOSED: &str = "deadline test signal closed";
const TEST_BOOT_ID: &str = "managed-deadline-boot";
const TEST_CONTROLLER_GENERATION: u64 = 1;
const TEST_HOST_PID: u32 = 1;
const TEST_WALL_TIMEOUT: Duration = Duration::from_secs(2);
const CUSTOM_CLIENT_TIMEOUT: Duration = Duration::from_secs(7);

async fn receive<T>(receiver: &mut oneshot::Receiver<T>) -> T {
    let deadline = WallInstant::now() + TEST_WALL_TIMEOUT;
    loop {
        match receiver.try_recv() {
            Ok(value) => return value,
            Err(oneshot::error::TryRecvError::Closed) => panic!("{SIGNAL_CLOSED}"),
            Err(oneshot::error::TryRecvError::Empty) => {}
        }
        assert!(WallInstant::now() < deadline);
        tokio::task::yield_now().await;
    }
}

async fn join<T>(task: JoinHandle<T>) -> T {
    let deadline = WallInstant::now() + TEST_WALL_TIMEOUT;
    while !task.is_finished() {
        assert!(WallInstant::now() < deadline);
        tokio::task::yield_now().await;
    }
    task.await.unwrap()
}

fn status(home: &Path) -> HostStatus {
    HostStatus {
        protocol_version: PROTOCOL_VERSION,
        boot_id: TEST_BOOT_ID.into(),
        host_pid: TEST_HOST_PID,
        controller_generation: TEST_CONTROLLER_GENERATION,
        home: home.to_path_buf(),
        capabilities: Vec::new(),
    }
}

#[tokio::test(start_paused = true)]
async fn status_connect_and_request_obey_one_preaged_deadline() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(STATUS_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let (request_sent, mut request_received) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = crate::rpc_test_support::Peer::new(stream);
        let request = stream.request().await.unwrap();
        request_sent.send(request).unwrap();
        pending::<()>().await;
    });
    let started = tokio::time::Instant::now();
    let deadline = ClientDeadline::test_after(DEFAULT_CLIENT_OPERATION_TIMEOUT);
    let first_stage = DEFAULT_CLIENT_OPERATION_TIMEOUT / DEADLINE_STAGE_COUNT;
    tokio::time::advance(first_stage).await;
    let client = Client::new(socket);
    let operation = tokio::spawn(async move { client.status_until(deadline).await });
    let request = receive(&mut request_received).await;
    assert_eq!(request.method, method::HOST_STATUS);

    tokio::time::advance(DEFAULT_CLIENT_OPERATION_TIMEOUT - first_stage).await;
    assert!(matches!(
        join(operation).await,
        Err(ClientError::Protocol(ProtocolError::OutcomeUnknown))
    ));
    assert_eq!(
        tokio::time::Instant::now() - started,
        DEFAULT_CLIENT_OPERATION_TIMEOUT
    );
    server.abort();
    let _ = server.await;
}

#[tokio::test(start_paused = true)]
async fn configured_client_timeout_controls_the_complete_request() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(CUSTOM_STATUS_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let (request_sent, mut request_received) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = crate::rpc_test_support::Peer::new(stream);
        let request = stream.request().await.unwrap();
        request_sent.send(request).unwrap();
        pending::<()>().await;
    });
    let started = tokio::time::Instant::now();
    let client = Client::new(socket).with_operation_timeout(CUSTOM_CLIENT_TIMEOUT);
    let operation = tokio::spawn(async move { client.status().await });
    let request = receive(&mut request_received).await;
    assert_eq!(request.method, method::HOST_STATUS);

    tokio::time::advance(CUSTOM_CLIENT_TIMEOUT).await;
    assert!(matches!(
        join(operation).await,
        Err(ClientError::Protocol(ProtocolError::OutcomeUnknown))
    ));
    assert_eq!(tokio::time::Instant::now() - started, CUSTOM_CLIENT_TIMEOUT);
    server.abort();
    let _ = server.await;
}

#[tokio::test(start_paused = true)]
async fn managed_snapshot_auth_and_business_requests_share_one_deadline() {
    let home = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(SNAPSHOT_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let (auth_sent, mut auth_received) = oneshot::channel();
    let (release_auth, auth_release) = oneshot::channel();
    let (snapshot_sent, mut snapshot_received) = oneshot::channel();
    let expected_home = home.path().to_path_buf();
    let server = tokio::spawn(async move {
        let (session_stream, _) = listener.accept().await.unwrap();
        let mut session_stream = crate::rpc_test_support::Peer::new(session_stream);
        let auth = session_stream.request().await.unwrap();
        auth_sent.send(auth.clone()).unwrap();
        auth_release.await.unwrap();
        let auth_response = ResponseEnvelope::success(&auth, &status(&expected_home)).unwrap();
        session_stream.respond(&auth_response).await.unwrap();
        let request = session_stream.request().await.unwrap();
        assert_eq!(request.method, method::GET_SESSION);
        let response = response(&request, MatrixScenario::Standard);
        session_stream.respond(&response).await.unwrap();
        let (snapshot_stream, _) = listener.accept().await.unwrap();
        let mut snapshot_stream = crate::rpc_test_support::Peer::new(snapshot_stream);
        let auth = snapshot_stream.request().await.unwrap();
        assert_eq!(auth.method, method::HOST_STATUS);
        let auth_response = ResponseEnvelope::success(&auth, &status(&expected_home)).unwrap();
        snapshot_stream.respond(&auth_response).await.unwrap();
        let request = snapshot_stream.request().await.unwrap();
        snapshot_sent.send(request).unwrap();
        pending::<()>().await;
    });
    let started = tokio::time::Instant::now();
    let deadline = ClientDeadline::test_after(DEFAULT_CLIENT_OPERATION_TIMEOUT);
    let client = Client::managed_local(home.path(), socket).unwrap();
    let operation =
        tokio::spawn(async move { client.snapshot_until(test_session_id(), deadline).await });
    let auth = receive(&mut auth_received).await;
    assert_eq!(auth.method, method::HOST_STATUS);
    let first_stage = DEFAULT_CLIENT_OPERATION_TIMEOUT / DEADLINE_STAGE_COUNT;
    tokio::time::advance(first_stage).await;
    release_auth.send(()).unwrap();
    let snapshot = receive(&mut snapshot_received).await;
    assert_eq!(snapshot.method, method::GET_SNAPSHOT);

    tokio::time::advance(DEFAULT_CLIENT_OPERATION_TIMEOUT - first_stage).await;
    assert!(matches!(
        join(operation).await,
        Err(ClientError::Protocol(ProtocolError::OutcomeUnknown))
    ));
    assert_eq!(
        tokio::time::Instant::now() - started,
        DEFAULT_CLIENT_OPERATION_TIMEOUT
    );
    server.abort();
    let _ = server.await;
}
