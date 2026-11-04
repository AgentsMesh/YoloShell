use super::constants::*;
use uuid::Uuid;
use yolo_agent_adapter_contract::constants::BINDING_SCHEMA_VERSION;
use yolo_agent_contract::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use yolo_terminal_model::{
    BootstrapDelivery, RecoveryState, RuntimeView, TerminalManifest, TerminalRuntimeStatus, TerminalView,
};

pub fn view() -> TerminalView {
    let terminal_id = Uuid::new_v4();
    TerminalView {
        manifest: TerminalManifest {
            schema_version: yolo_terminal_model::PROTOCOL_VERSION,
            terminal_id: terminal_id,
            name: SESSION_NAME.into(),
            command: SHELL_EXECUTABLE.into(),
            args: Vec::new(),
            cwd: WORKING_DIRECTORY.into(),
            cols: yolo_terminal_model::DEFAULT_COLS,
            rows: yolo_terminal_model::DEFAULT_ROWS,
            created_at_ms: DETECTED_MS,
            updated_at_ms: UPDATED_MS,
            archived: false,
            recovery_state: RecoveryState::Created,
        },
        runtime: Some(RuntimeView {
            presentation: Default::default(),
            runtime_epoch: Uuid::new_v4(),
            worker_id: Uuid::new_v4(),
            worker_pid: WORKER_PID,
            worker_start_time: WORKER_START,
            child_pid: SHELL_PID,
            child_start_time: SHELL_START,
            status: TerminalRuntimeStatus::Running,
            last_output_sequence: 0,
            exit: None,
            bootstrap_delivery: BootstrapDelivery::NotRequired,
            capabilities: None,
            fault_reap: None,
        }),
        agent: Some(AgentBinding {
            schema_version: BINDING_SCHEMA_VERSION,
            terminal_id: terminal_id,
            kind: AgentKind::Codex,
            provider_session_id: PROVIDER_REFERENCE.into(),
            executable: PROVIDER_EXECUTABLE.into(),
            cwd: WORKING_DIRECTORY.into(),
            evidence_path: PROVIDER_EVIDENCE.into(),
            data_home: PROVIDER_DATA_HOME.into(),
            process_pid: Some(PROVIDER_PID),
            process_start_time: Some(PROVIDER_START),
            state: AgentState::Running,
            recovery_availability: AgentRecoveryAvailability::Available,
            detected_at_ms: DETECTED_MS,
            updated_at_ms: UPDATED_MS,
        }),
        completed_attachment: None,
    }
}

pub fn exit(view: &mut TerminalView) {
    let binding = view.agent.as_mut().unwrap();
    binding.state = AgentState::Exited;
    binding.process_pid = None;
    binding.process_start_time = None;
    binding.updated_at_ms += IDENTITY_STEP as i64;
}
