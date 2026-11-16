use super::constants::{DEFAULT_CLIENT_OPERATION_TIMEOUT, request_operation_timeout};
use super::*;
use crate::protocol::{RequestEnvelope, method};
use std::future::pending;
use std::path::Path;
use tokio::net::UnixListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const CREATE_SOCKET_NAME: &str = "create-startup-deadline.sock";
const RESUME_SOCKET_NAME: &str = "resume-startup-deadline.sock";
const CONNECTION_RESUME_SOCKET_NAME: &str = "connection-resume-startup-deadline.sock";
const TEST_CREATION_TOKEN_VALUE: u128 = 101;
const TEST_SESSION_ID_VALUE: u128 = 103;
const TEST_WORKING_DIRECTORY: &str = "/tmp/yolo-startup-deadline";

fn pending_server(
    socket: &Path,
    expected_method: &'static str,
) -> (JoinHandle<()>, oneshot::Receiver<()>) {
    let listener = UnixListener::bind(socket).unwrap();
    let (request_sent, request_received) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = crate::rpc_test_support::Peer::new(stream);
        let request = stream.request().await.unwrap();
        assert_eq!(request.method, expected_method);
        request_sent.send(()).unwrap();
        pending::<()>().await;
        drop(stream);
    });
    (server, request_received)
}

async fn assert_operation_floor<T>(
    mut operation: JoinHandle<Result<T, ClientError>>,
    server: JoinHandle<()>,
    request_received: oneshot::Receiver<()>,
    method_name: &str,
) {
    let started_at = tokio::time::Instant::now();
    let startup_floor =
        request_operation_timeout(DEFAULT_CLIENT_OPERATION_TIMEOUT, method::CREATE_SESSION);
    let floor = request_operation_timeout(DEFAULT_CLIENT_OPERATION_TIMEOUT, method_name);
    let checkpoint = if floor > startup_floor {
        startup_floor
    } else {
        DEFAULT_CLIENT_OPERATION_TIMEOUT
    };
    crate::rpc_test_support::scheduled(request_received).await;
    assert!(
        tokio::time::timeout_at(started_at + checkpoint, &mut operation)
            .await
            .is_err()
    );
    assert!(matches!(
        operation.await.unwrap(),
        Err(ClientError::Protocol(ProtocolError::OutcomeUnknown))
    ));
    assert_eq!(tokio::time::Instant::now() - started_at, floor);
    server.abort();
    let _ = server.await;
}

#[tokio::test(start_paused = true)]
async fn create_session_uses_the_session_startup_deadline_floor() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(CREATE_SOCKET_NAME);
    let (server, request_received) = pending_server(&socket, method::CREATE_SESSION);
    let client = Client::new(socket);
    let operation = tokio::spawn(async move {
        client
            .create_session(&CreateTerminal {
                creation_token: Uuid::from_u128(TEST_CREATION_TOKEN_VALUE),
                name: None,
                cwd: PathBuf::from(TEST_WORKING_DIRECTORY),
                cols: None,
                rows: None,
                initial_input: None,
                context: None,
            })
            .await
    });

    assert_operation_floor(operation, server, request_received, method::CREATE_SESSION).await;
}

#[tokio::test(start_paused = true)]
async fn resume_uses_the_session_resume_deadline_floor() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(RESUME_SOCKET_NAME);
    let (server, request_received) = pending_server(&socket, method::RESUME_SESSION_EXACT);
    let client = Client::new(socket);
    let operation = tokio::spawn(async move {
        client
            .resume(Uuid::from_u128(TEST_SESSION_ID_VALUE), true)
            .await
    });

    assert_operation_floor(
        operation,
        server,
        request_received,
        method::RESUME_SESSION_EXACT,
    )
    .await;
}

#[tokio::test(start_paused = true)]
async fn rpc_connection_resume_uses_the_session_resume_deadline_floor() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(CONNECTION_RESUME_SOCKET_NAME);
    let (server, request_received) = pending_server(&socket, method::RESUME_SESSION_EXACT);
    let client = Client::new(socket);
    let mut connection = client.connect().await.unwrap();
    let operation = tokio::spawn(async move {
        let request = ResumeRequest {
            selection: TerminalLifecycleSelection::unconditional(Uuid::from_u128(
                TEST_SESSION_ID_VALUE,
            )),
            recreate: true,
        };
        connection
            .request::<_, TerminalView>(method::RESUME_SESSION_EXACT, &request)
            .await
    });

    assert_operation_floor(
        operation,
        server,
        request_received,
        method::RESUME_SESSION_EXACT,
    )
    .await;
}
