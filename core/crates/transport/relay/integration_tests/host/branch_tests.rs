use super::*;
use crate::model::{
    BootstrapDelivery, HostStatus, PROTOCOL_VERSION, RecoveryState, RuntimeView, TerminalManifest,
    TerminalRuntimeStatus, TerminalView,
};
use crate::relay::test_support::{FakeHost, Reply};
use serde::Serialize;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use yolo_runtime_rpc_testing::Peer;

const HOST_SOCKET_NAME: &str = "host-start-race.sock";
const TRUE_EXECUTABLE: &str = crate::test_executables::TRUE;
const NEVER_SPAWN_EXECUTABLE: &str = "/missing/yolo-host-must-not-spawn";
const TEST_HOME: &str = "/tmp";
const PRIVATE_SOCKET_MODE: u32 = 0o600;
const TEST_SCHEMA_VERSION: u16 = 1;
const TEST_COLUMNS: u16 = 80;
const TEST_ROWS: u16 = 24;
const INITIAL_OUTPUT_SEQUENCE: u64 = 0;
const TEST_HOST_PID: u32 = 31;
const TEST_CONTROLLER_GENERATION: u64 = 37;
const TEST_WORKER_PID: u32 = 41;
const TEST_CHILD_PID: u32 = 43;
const TEST_TIMESTAMP_MS: i64 = 1;

#[tokio::test]
async fn prepare_reports_a_host_that_becomes_ready_after_the_initial_probe() {
    let home = tempfile::tempdir().unwrap();
    let sockets = tempfile::tempdir().unwrap();
    let socket = sockets.path().join(HOST_SOCKET_NAME);
    let initial = UnixListener::bind(&socket).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(PRIVATE_SOCKET_MODE)).unwrap();
    let id = Uuid::new_v4();
    let expected_home = home.path().to_path_buf();
    let server_socket = socket.clone();
    let server = tokio::spawn(async move {
        let (rejected, _) = initial.accept().await.unwrap();
        let mut rejected = Peer::new(rejected);
        rejected.request().await.unwrap();
        drop(rejected);
        drop(initial);
        fs::remove_file(&server_socket).unwrap();
        let ready = UnixListener::bind(&server_socket).unwrap();
        fs::set_permissions(
            &server_socket,
            fs::Permissions::from_mode(PRIVATE_SOCKET_MODE),
        )
        .unwrap();
        let (status_stream, _) = ready.accept().await.unwrap();
        let mut status_stream = Peer::new(status_stream);
        reply(&mut status_stream, &status(&expected_home)).await;
        let (session_stream, _) = ready.accept().await.unwrap();
        let mut session_stream = Peer::new(session_stream);
        reply(&mut session_stream, &status(&expected_home)).await;
        reply(&mut session_stream, &session(id, TerminalRuntimeStatus::Running)).await;
    });
    let runtime =
        RelayRuntime::managed(home.path(), &socket, Some(TRUE_EXECUTABLE.into())).unwrap();
    let (events, mut statuses) = mpsc::unbounded_channel();

    runtime.prepare(id, &events).await.unwrap();

    assert_eq!(
        statuses.try_recv().unwrap(),
        RelayStatus::HostStarted {
            executable: TRUE_EXECUTABLE.into(),
        }
    );
    assert!(statuses.try_recv().is_err());
    server.await.unwrap();
}

#[tokio::test]
async fn managed_runtime_propagates_identity_errors_without_spawning() {
    let home = tempfile::tempdir().unwrap();
    let other_home = tempfile::tempdir().unwrap();
    let host = FakeHost::start(vec![Reply::value(&status(other_home.path()))]).await;
    let runtime = RelayRuntime::managed(
        home.path(),
        host.socket(),
        Some(NEVER_SPAWN_EXECUTABLE.into()),
    )
    .unwrap();
    let (events, _) = mpsc::unbounded_channel();

    assert!(matches!(
        runtime.prepare(Uuid::new_v4(), &events).await,
        Err(RelayError::Client(ClientError::HostHomeMismatch { .. }))
    ));
}

#[tokio::test]
async fn pending_finalization_is_rejected_before_resume() {
    let id = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::value(&status(Path::new(TEST_HOME))),
        Reply::value(&session(id, TerminalRuntimeStatus::Finalizing)),
    ])
    .await;
    let runtime = RelayRuntime::existing(host.client());
    let (events, _) = mpsc::unbounded_channel();

    assert!(matches!(
        runtime.prepare(id, &events).await,
        Err(RelayError::SessionNotAttachable(actual)) if actual == id
    ));
}

async fn reply(stream: &mut Peer, value: &impl Serialize) {
    let request = stream.request().await.unwrap();
    let response = crate::protocol::ResponseEnvelope::success(&request, value).unwrap();
    stream.respond(&response).await.unwrap();
}

fn status(home: &Path) -> HostStatus {
    HostStatus {
        protocol_version: PROTOCOL_VERSION,
        boot_id: String::new(),
        host_pid: TEST_HOST_PID,
        controller_generation: TEST_CONTROLLER_GENERATION,
        home: home.into(),
        capabilities: Vec::new(),
    }
}

fn session(id: Uuid, status: TerminalRuntimeStatus) -> TerminalView {
    TerminalView {
        manifest: TerminalManifest {
            schema_version: TEST_SCHEMA_VERSION,
            terminal_id: id,
            name: String::new(),
            command: String::new(),
            args: Vec::new(),
            cwd: TEST_HOME.into(),
            cols: TEST_COLUMNS,
            rows: TEST_ROWS,
            created_at_ms: TEST_TIMESTAMP_MS,
            updated_at_ms: TEST_TIMESTAMP_MS,
            archived: false,
            recovery_state: RecoveryState::Reattached,
        },
        runtime: Some(RuntimeView {
            presentation: Default::default(),
            runtime_epoch: Uuid::new_v4(),
            worker_id: Uuid::new_v4(),
            worker_pid: TEST_WORKER_PID,
            worker_start_time: TEST_WORKER_PID.into(),
            child_pid: TEST_CHILD_PID,
            child_start_time: TEST_CHILD_PID.into(),
            status,
            last_output_sequence: INITIAL_OUTPUT_SEQUENCE,
            exit: None,
            bootstrap_delivery: BootstrapDelivery::NotRequired,
            capabilities: Some(Vec::new()),
            fault_reap: None,
        }),
        agent: None,
        completed_attachment: None,
    }
}
