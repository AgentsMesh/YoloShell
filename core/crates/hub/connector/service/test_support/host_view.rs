use super::constants::*;
use yolo_hub_identity::RuntimeTarget;
use yolo_runtime::model::*;

pub fn view(target: RuntimeTarget) -> TerminalView {
    TerminalView {
        manifest: TerminalManifest {
            schema_version: MANIFEST_SCHEMA,
            terminal_id: target.address.terminal_id.as_uuid(),
            name: SESSION_NAME.into(),
            command: SESSION_SHELL.into(),
            args: Vec::new(),
            cwd: SESSION_DIRECTORY.into(),
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            created_at_ms: MODEL_CLOCK,
            updated_at_ms: MODEL_CLOCK,
            archived: false,
            recovery_state: RecoveryState::Created,
        },
        runtime: Some(RuntimeView {
            presentation: Default::default(),
            runtime_epoch: target.runtime_epoch.as_uuid(),
            worker_id: uuid::Uuid::new_v4(),
            worker_pid: MODEL_PROCESS,
            worker_start_time: MODEL_PROCESS_START,
            child_pid: MODEL_PROCESS,
            child_start_time: MODEL_PROCESS_START,
            status: TerminalRuntimeStatus::Running,
            last_output_sequence: INITIAL_OUTPUT_SEQUENCE,
            exit: None,
            bootstrap_delivery: BootstrapDelivery::NotRequired,
            capabilities: None,
            fault_reap: None,
        }),
        agent: None,
        completed_attachment: None,
    }
}

pub fn status(home: std::path::PathBuf) -> HostStatus {
    HostStatus {
        protocol_version: PROTOCOL_VERSION,
        boot_id: MODEL_BOOT_ID.into(),
        host_pid: MODEL_PROCESS,
        controller_generation: MODEL_CONTROLLER_GENERATION,
        home,
        capabilities: Vec::new(),
    }
}
