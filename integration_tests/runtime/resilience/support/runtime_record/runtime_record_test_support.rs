use super::runtime_record_authority::BoundRuntimeRecord;
use super::*;

const TEST_AUTH_TOKEN: &str = "trusted-cleanup-record";
const TEST_COMMAND: &str = "/bin/sleep";
const TEST_GENERATION: u64 = 1;
const TEST_SESSION_NAME: &str = "trusted-cleanup-session";

pub(super) fn bind_record(paths: &Paths, record: RuntimeRecord) -> BoundRuntimeRecord {
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    let trusted = runtime_view(&record);
    BoundRuntimeRecord::bind_runtime(paths, record.clone(), record.terminal_id, &trusted).unwrap()
}

pub(super) fn runtime_view(record: &RuntimeRecord) -> yolo_runtime::model::RuntimeView {
    yolo_runtime::model::RuntimeView {
        presentation: Default::default(),
        runtime_epoch: record.runtime_epoch,
        worker_id: record.worker_id,
        worker_pid: record.worker_pid,
        worker_start_time: record.worker_start_time,
        child_pid: record.child_pid,
        child_start_time: record.child_start_time,
        status: TerminalRuntimeStatus::Running,
        last_output_sequence: u64::default(),
        exit: None,
        bootstrap_delivery: record.bootstrap_delivery,
        capabilities: None,
        fault_reap: None,
    }
}

pub(super) fn session_view(record: &RuntimeRecord) -> yolo_runtime::TerminalView {
    yolo_runtime::TerminalView {
        manifest: yolo_runtime::TerminalManifest {
            schema_version: yolo_runtime::constants::SESSION_MANIFEST_SCHEMA_VERSION,
            terminal_id: record.terminal_id,
            name: TEST_SESSION_NAME.into(),
            command: record.command.clone(),
            args: record.args.clone(),
            cwd: record.cwd.clone(),
            cols: record.cols,
            rows: record.rows,
            created_at_ms: record.started_at_ms,
            updated_at_ms: record.started_at_ms,
            archived: false,
            recovery_state: yolo_runtime::RecoveryState::Created,
        },
        runtime: Some(runtime_view(record)),
        agent: None,
        completed_attachment: None,
    }
}

pub(super) fn record_for_identity(
    paths: &Paths,
    terminal_id: Uuid,
    identity: ProcessIdentity,
) -> RuntimeRecord {
    paths.ensure_session(terminal_id).unwrap();
    RuntimeRecord {
        schema_version: yolo_runtime::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: identity.pid,
        worker_start_time: identity.start_time,
        child_pid: identity.pid,
        child_start_time: identity.start_time,
        boot_id: yolo_process_test_support::boot_id(),
        socket_path: paths.worker_socket(terminal_id).unwrap(),
        auth_token: TEST_AUTH_TOKEN.into(),
        controller_generation: TEST_GENERATION,
        started_at_ms: i64::default(),
        command: TEST_COMMAND.into(),
        args: Vec::new(),
        cwd: paths.home().to_path_buf(),
        cols: yolo_runtime::model::MIN_TERMINAL_COLUMNS,
        rows: yolo_runtime::model::MIN_TERMINAL_ROWS,
        bootstrap_delivery: yolo_runtime::model::BootstrapDelivery::NotRequired,
    }
}
