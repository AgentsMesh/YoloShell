use super::*;
use std::future::pending;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant as WallInstant};
use tokio::net::UnixListener;
use tokio::sync::oneshot;
use yolo_client::ClientError;
use yolo_remote_broker::EXISTING_SOCKET_CLIENT_OPERATION_TIMEOUT;
use yolo_runtime::protocol::{ProtocolError, RequestEnvelope};

const BROKER_SOCKET_NAME: &str = "remote-deadline.sock";
const HOLDER_ID: &str = "remote-deadline-holder";
const PRIVATE_SOCKET_MODE: u32 = 0o600;
const SESSION_ID: u128 = 41;
const PRE_DEADLINE_MARGIN: Duration = Duration::from_millis(1);
const REQUEST_OBSERVE_TIMEOUT: Duration = Duration::from_secs(2);
const REQUEST_SIGNAL_CLOSED: &str = "remote deadline request signal closed";

#[tokio::test(start_paused = true)]
async fn broker_runtime_preserves_the_remote_existing_socket_budget() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(BROKER_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    std::fs::set_permissions(
        &socket,
        std::fs::Permissions::from_mode(PRIVATE_SOCKET_MODE),
    )
    .unwrap();
    let (request_seen, mut request_observed) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
        let request = stream.request().await.unwrap();
        request_seen.send(request).unwrap();
        pending::<()>().await;
    });
    let runtime = relay_runtime(&Cli {
        home: None,
        socket: None,
        broker_socket: Some(socket),
        terminal_id: uuid::Uuid::from_u128(SESSION_ID),
        holder_id: HOLDER_ID.to_owned(),
        host_executable: None,
        take_control: false,
        status_fd: None,
        after_seq: INITIAL_OUTPUT_SEQUENCE,
    })
    .unwrap();
    let operation = tokio::spawn(async move { runtime.client().status().await });
    receive_request(&mut request_observed).await;

    tokio::time::advance(EXISTING_SOCKET_CLIENT_OPERATION_TIMEOUT - PRE_DEADLINE_MARGIN).await;
    assert!(!operation.is_finished());
    tokio::time::advance(PRE_DEADLINE_MARGIN).await;
    assert!(matches!(
        operation.await.unwrap(),
        Err(ClientError::Protocol(ProtocolError::OutcomeUnknown))
    ));
    server.abort();
    let _ = server.await;
}

async fn receive_request(receiver: &mut oneshot::Receiver<RequestEnvelope>) {
    let deadline = WallInstant::now() + REQUEST_OBSERVE_TIMEOUT;
    loop {
        match receiver.try_recv() {
            Ok(_) => return,
            Err(oneshot::error::TryRecvError::Closed) => panic!("{REQUEST_SIGNAL_CLOSED}"),
            Err(oneshot::error::TryRecvError::Empty) => {}
        }
        assert!(WallInstant::now() < deadline);
        tokio::task::yield_now().await;
    }
}
