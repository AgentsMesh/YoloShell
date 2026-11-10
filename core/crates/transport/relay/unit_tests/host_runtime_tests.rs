use super::*;
use crate::model::{
    HostStatus, PROTOCOL_VERSION, RecoveryState, RuntimeView, TerminalManifest, TerminalRuntimeStatus,
    TerminalView,
};
use crate::protocol::ErrorBody;
use crate::protocol::error_code::INTERNAL as INTERNAL_CODE;
use crate::relay::test_support::{FakeHost, Reply};
use tokio::sync::mpsc;

const TEST_WORKER_PID: u32 = 11;
const TEST_CHILD_PID: u32 = 12;

#[test]
fn client_deadline_is_a_recoverable_transport_failure() {
    assert!(is_transport_error(&ClientError::Deadline));
}

fn status() -> HostStatus {
    HostStatus {
        protocol_version: PROTOCOL_VERSION,
        boot_id: "boot".into(),
        host_pid: 10,
        controller_generation: 2,
        home: PathBuf::from("/tmp/yolo"),
        capabilities: Vec::new(),
    }
}

fn session(id: Uuid, runtime: Option<TerminalRuntimeStatus>, archived: bool) -> TerminalView {
    TerminalView {
        manifest: TerminalManifest {
            schema_version: 1,
            terminal_id: id,
            name: "session".into(),
            command: "/bin/sh".into(),
            args: Vec::new(),
            cwd: PathBuf::from("/tmp"),
            cols: 80,
            rows: 24,
            created_at_ms: 1,
            updated_at_ms: 1,
            archived,
            recovery_state: RecoveryState::NeedsRecreate,
        },
        runtime: runtime.map(|status| RuntimeView {
            presentation: Default::default(),
            runtime_epoch: Uuid::new_v4(),
            worker_id: Uuid::new_v4(),
            worker_pid: TEST_WORKER_PID,
            worker_start_time: TEST_WORKER_PID as u64,
            child_pid: TEST_CHILD_PID,
            child_start_time: TEST_CHILD_PID as u64,
            status,
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

#[tokio::test]
async fn prepare_attaches_running_and_rejects_archived_sessions() {
    let id = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::value(&status()),
        Reply::value(&session(id, Some(TerminalRuntimeStatus::Running), false)),
    ])
    .await;
    let runtime = RelayRuntime::existing(host.client());
    let (events, mut statuses) = mpsc::unbounded_channel();
    runtime.prepare(id, &events).await.unwrap();
    assert!(statuses.try_recv().is_err());

    let host = FakeHost::start(vec![
        Reply::value(&status()),
        Reply::value(&session(id, None, true)),
    ])
    .await;
    let runtime = RelayRuntime::existing(host.client());
    assert!(matches!(
        runtime.prepare(id, &events).await,
        Err(RelayError::ArchivedSession(actual)) if actual == id
    ));
}

#[tokio::test]
async fn prepare_resumes_non_running_sessions_and_reports_the_transition() {
    let id = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::value(&status()),
        Reply::value(&session(id, Some(TerminalRuntimeStatus::Unreachable), false)),
        Reply::value(&session(id, Some(TerminalRuntimeStatus::Running), false)),
    ])
    .await;
    let runtime = RelayRuntime::existing(host.client());
    let (events, mut statuses) = mpsc::unbounded_channel();
    runtime.prepare(id, &events).await.unwrap();
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::SessionResumed);
}

#[tokio::test]
async fn concurrent_resume_accepts_the_winner_and_rejects_non_attachable_results() {
    let id = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::value(&status()),
        Reply::value(&session(id, Some(TerminalRuntimeStatus::Unreachable), false)),
        Reply::Error(ErrorBody::conflict("already running")),
        Reply::value(&session(id, Some(TerminalRuntimeStatus::Running), false)),
    ])
    .await;
    let runtime = RelayRuntime::existing(host.client());
    let (events, _) = mpsc::unbounded_channel();
    runtime.prepare(id, &events).await.unwrap();

    let host = FakeHost::start(vec![
        Reply::value(&status()),
        Reply::value(&session(id, None, false)),
        Reply::value(&session(id, Some(TerminalRuntimeStatus::Exited), false)),
    ])
    .await;
    let runtime = RelayRuntime::existing(host.client());
    assert!(matches!(
        runtime.prepare(id, &events).await,
        Err(RelayError::SessionNotAttachable(actual)) if actual == id
    ));
}

#[tokio::test]
async fn prepare_propagates_resume_and_non_transport_host_errors() {
    let id = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::value(&status()),
        Reply::value(&session(id, None, false)),
        Reply::error(INTERNAL_CODE),
    ])
    .await;
    let runtime = RelayRuntime::existing(host.client());
    let (events, _) = mpsc::unbounded_channel();
    assert!(matches!(
        runtime.prepare(id, &events).await,
        Err(RelayError::Client(_))
    ));

    let host = FakeHost::start(vec![Reply::error(INTERNAL_CODE)]).await;
    let runtime = RelayRuntime::existing(host.client());
    assert!(matches!(
        runtime.prepare(id, &events).await,
        Err(RelayError::Client(_))
    ));
}

#[tokio::test]
async fn existing_runtime_cannot_spawn_a_missing_host() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = RelayRuntime::existing(crate::client::Client::new(
        directory.path().join("missing.sock"),
    ));
    let (events, _) = mpsc::unbounded_channel();
    assert!(matches!(
        runtime.prepare(Uuid::new_v4(), &events).await,
        Err(RelayError::Client(ClientError::Protocol(_)))
    ));
}

#[test]
fn managed_runtime_keeps_explicit_and_companion_launch_contracts() {
    let host = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(FakeHost::start(Vec::new()));
    let explicit = PathBuf::from("/tmp/custom-yolo-host");
    let managed =
        RelayRuntime::managed("/tmp/home", host.socket(), Some(explicit.clone())).unwrap();
    assert_eq!(managed.launch.as_ref().unwrap().executable, explicit);
    assert_eq!(managed.client().socket_path(), host.socket());
    let converted = RelayRuntime::from(host.client());
    assert!(converted.launch.is_none());
    assert_eq!(converted.client().socket_path(), host.socket());
}
