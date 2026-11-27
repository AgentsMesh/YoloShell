use super::*;
use crate::client::ClientError;
use crate::model::{
    HostStatus, PROTOCOL_VERSION, RecoveryState, RuntimeView, TerminalManifest, TerminalRuntimeStatus,
    TerminalView,
};
use crate::protocol::{ResponseEnvelope, method};
use crate::relay::{RelayCancellation, RelayOptions, relay_session_with_status_and_cancellation};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_runtime_rpc_testing::Peer;

const LATE_SOCKET_DELAY: std::time::Duration = std::time::Duration::from_millis(75);
const CANCELLED_ACCEPT_WINDOW: std::time::Duration = std::time::Duration::from_millis(100);
const PRIVATE_SOCKET_MODE: u32 = 0o600;
const REJECTED_SOCKET_NAME: &str = "rejected-runtime.sock";
const REJECTED_STATUS_CODE: &str = "status-rejected";
const NON_DIRECTORY_PARENT: &str = "not-a-directory";
const NESTED_SOCKET_NAME: &str = "nested.sock";
const REGULAR_FILE_BYTES: &[u8] = b"file";
const CANCELLED_RESIZE_CHANNEL_CAPACITY: usize = 1;

const TEST_WORKER_PID: u32 = 10;
const TEST_CHILD_PID: u32 = 11;

fn running_session(id: Uuid) -> TerminalView {
    TerminalView {
        manifest: TerminalManifest {
            schema_version: 1,
            terminal_id: id,
            name: "shell".into(),
            command: "/bin/sh".into(),
            args: vec![],
            cwd: "/tmp".into(),
            cols: 80,
            rows: 24,
            created_at_ms: 1,
            updated_at_ms: 1,
            archived: false,
            recovery_state: RecoveryState::Reattached,
        },
        runtime: Some(RuntimeView {
            presentation: Default::default(),
            runtime_epoch: Uuid::new_v4(),
            worker_id: Uuid::new_v4(),
            worker_pid: TEST_WORKER_PID,
            worker_start_time: TEST_WORKER_PID as u64,
            child_pid: TEST_CHILD_PID,
            child_start_time: TEST_CHILD_PID as u64,
            status: TerminalRuntimeStatus::Running,
            last_output_sequence: 0,
            exit: None,
            bootstrap_delivery: crate::model::BootstrapDelivery::NotRequired,
            capabilities: Some(Vec::new()),
            fault_reap: None,
        }),
        agent: None,
        completed_attachment: None,
    }
}

fn status() -> HostStatus {
    HostStatus {
        protocol_version: PROTOCOL_VERSION,
        boot_id: "boot".into(),
        host_pid: 9,
        controller_generation: 1,
        home: "/var/lib/yolo".into(),
        capabilities: vec![],
    }
}

async fn serve_late(path: std::path::PathBuf, id: Uuid) {
    tokio::time::sleep(LATE_SOCKET_DELAY).await;
    let listener = UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_SOCKET_MODE)).unwrap();
    for _ in 0..2 {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = Peer::new(stream);
        let request = stream.request().await.unwrap();
        let response = if request.method == method::HOST_STATUS {
            ResponseEnvelope::success(&request, &status()).unwrap()
        } else {
            ResponseEnvelope::success(&request, &running_session(id)).unwrap()
        };
        stream.respond(&response).await.unwrap();
    }
}

#[tokio::test]
async fn broker_runtime_waits_for_late_socket_without_spawning_local_host() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("runtime.sock");
    let id = Uuid::new_v4();
    let server = tokio::spawn(serve_late(socket.clone(), id));
    let runtime = RelayRuntime::broker(Client::new(&socket)).unwrap();
    let (events, _) = mpsc::unbounded_channel();
    runtime.prepare(id, &events).await.unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn cancellation_stops_late_broker_wait_without_leaking_retries() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("runtime.sock");
    let runtime = RelayRuntime::broker(Client::new(&socket)).unwrap();
    let cancellation = RelayCancellation::default();
    let cancel = cancellation.clone();
    let (status_sender, _) = mpsc::unbounded_channel();
    let (_resize, resize_events) = mpsc::channel(CANCELLED_RESIZE_CHANNEL_CAPACITY);
    let options = RelayOptions::new(Uuid::new_v4(), "holder").unwrap();
    let task = tokio::spawn(relay_session_with_status_and_cancellation(
        runtime,
        options,
        tokio::io::empty(),
        tokio::io::sink(),
        resize_events,
        status_sender,
        cancellation,
    ));
    tokio::time::sleep(LATE_SOCKET_DELAY).await;
    cancel.cancel();
    assert!(matches!(task.await.unwrap(), Err(RelayError::Cancelled)));

    let listener = UnixListener::bind(&socket).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(PRIVATE_SOCKET_MODE)).unwrap();
    assert!(
        tokio::time::timeout(CANCELLED_ACCEPT_WINDOW, listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn unsafe_broker_paths_and_endpoints_fail_immediately() {
    assert!(RelayRuntime::broker(Client::new("relative.sock")).is_err());
    let root = TempDir::new().unwrap();
    let socket = root.path().join("runtime.sock");
    fs::write(&socket, b"not-a-socket").unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(PRIVATE_SOCKET_MODE)).unwrap();
    let runtime = RelayRuntime::broker(Client::new(socket)).unwrap();
    assert!(matches!(
        runtime.wait_for_broker().await,
        Err(RelayError::Io(_))
    ));

    let parent = root.path().join(NON_DIRECTORY_PARENT);
    fs::write(&parent, REGULAR_FILE_BYTES).unwrap();
    assert!(matches!(
        validate_waiting_socket(&parent.join(NESTED_SOCKET_NAME)),
        Err(RelayError::Io(_))
    ));
}

#[tokio::test]
async fn broker_wait_propagates_a_non_transport_status_failure() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join(REJECTED_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(PRIVATE_SOCKET_MODE)).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = Peer::new(stream);
        let request = stream.request().await.unwrap();
        let response = ResponseEnvelope::failure(
            &request,
            crate::protocol::ErrorBody::internal(REJECTED_STATUS_CODE),
        );
        stream.respond(&response).await.unwrap();
    });
    let runtime = RelayRuntime::broker(Client::new(&socket)).unwrap();

    assert!(matches!(
        runtime.wait_for_broker().await,
        Err(RelayError::Client(ClientError::Remote { .. }))
    ));
    server.await.unwrap();
}
