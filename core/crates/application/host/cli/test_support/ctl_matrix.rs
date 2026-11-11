const CTL_MATRIX_RUNTIME_EPOCH_VALUE: u128 = 1;
const CTL_MATRIX_WORKER_ID_VALUE: u128 = 2;
const CTL_MATRIX_WORKER_PID: u32 = 10;
const CTL_MATRIX_CHILD_PID: u32 = 11;
const CTL_MATRIX_PROCESS_START: u64 = 12;
const CTL_MATRIX_TIMESTAMP: i64 = 13;
const CTL_MATRIX_SESSION_NAME: &str = "matrix-session";
const CTL_MATRIX_SHELL: &str = "/bin/sh";

async fn run_ctl_matrix_command(
    socket: &std::path::Path,
    command: &[&str],
) -> anyhow::Result<()> {
    run_command(Command::Ctl(parse_existing_socket_ctl(socket, command))).await
}

fn ctl_matrix_session() -> yolo_runtime::model::TerminalView {
    use yolo_runtime::model::{
        BootstrapDelivery, RecoveryState, RuntimeView, TerminalManifest, TerminalRuntimeStatus, TerminalView,
    };
    let terminal_id = uuid::Uuid::parse_str(SESSION_ID).unwrap();
    TerminalView {
        manifest: TerminalManifest {
            schema_version: yolo_runtime::constants::SESSION_MANIFEST_SCHEMA_VERSION,
            terminal_id: terminal_id,
            name: CTL_MATRIX_SESSION_NAME.into(),
            command: CTL_MATRIX_SHELL.into(),
            args: Vec::new(),
            cwd: std::path::PathBuf::from(CTL_MATRIX_CWD),
            cols: yolo_runtime::model::DEFAULT_COLS,
            rows: yolo_runtime::model::DEFAULT_ROWS,
            created_at_ms: CTL_MATRIX_TIMESTAMP,
            updated_at_ms: CTL_MATRIX_TIMESTAMP,
            archived: false,
            recovery_state: RecoveryState::Created,
        },
        runtime: Some(RuntimeView {
            presentation: Default::default(),
            runtime_epoch: uuid::Uuid::from_u128(CTL_MATRIX_RUNTIME_EPOCH_VALUE),
            worker_id: uuid::Uuid::from_u128(CTL_MATRIX_WORKER_ID_VALUE),
            worker_pid: CTL_MATRIX_WORKER_PID,
            worker_start_time: CTL_MATRIX_PROCESS_START,
            child_pid: CTL_MATRIX_CHILD_PID,
            child_start_time: CTL_MATRIX_PROCESS_START,
            status: TerminalRuntimeStatus::Running,
            last_output_sequence: yolo_relay::INITIAL_OUTPUT_SEQUENCE,
            exit: None,
            bootstrap_delivery: BootstrapDelivery::NotRequired,
            capabilities: Some(Vec::new()),
            fault_reap: None,
        }),
        agent: None,
        completed_attachment: None,
    }
}
