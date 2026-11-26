use super::constants::*;
use yolo_application_support::constants::{
    RUNTIME_RECORD_SCHEMA_VERSION, SESSION_MANIFEST_SCHEMA_VERSION, WORKER_LAUNCH_SCHEMA_VERSION,
};
use yolo_paths::Paths;
use yolo_terminal_model::{
    BootstrapDelivery, DEFAULT_COLS, DEFAULT_ROWS, RecoveryState, RuntimeRecord, TerminalManifest,
    WorkerLaunchSpec,
};
use yolo_startup_checkpoint_contract::{
    StartupCheckpointEvidence, StartupLaunchFence, StartupPhase, StartupProcessIdentity,
    StartupStage, StartupStatus, constants::STARTUP_CHECKPOINT_SCHEMA_VERSION,
};

pub fn launch(paths: &Paths) -> WorkerLaunchSpec {
    let terminal_id = uuid::Uuid::new_v4();
    WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: terminal_id,
        runtime_epoch: uuid::Uuid::new_v4(),
        worker_id: uuid::Uuid::new_v4(),
        socket_path: paths.worker_socket(terminal_id).unwrap(),
        auth_token: AUTH_TOKEN.into(),
        controller_generation: GENERATION,
        command: SHELL.into(),
        args: vec![],
        cwd: paths.home().to_owned(),
        environment: Default::default(),
        initial_input: None,
        cols: DEFAULT_COLS,
        rows: DEFAULT_ROWS,
        context: None,
    }
}
pub fn runtime(launch: &WorkerLaunchSpec) -> RuntimeRecord {
    RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: launch.terminal_id,
        runtime_epoch: launch.runtime_epoch,
        worker_id: launch.worker_id,
        worker_pid: WORKER_PID,
        worker_start_time: WORKER_START,
        child_pid: CHILD_PID,
        child_start_time: CHILD_START,
        boot_id: BOOT_ID.into(),
        socket_path: launch.socket_path.clone(),
        auth_token: launch.auth_token.clone(),
        controller_generation: GENERATION,
        started_at_ms: CLOCK,
        command: launch.command.clone(),
        args: launch.args.clone(),
        cwd: launch.cwd.clone(),
        cols: launch.cols,
        rows: launch.rows,
        bootstrap_delivery: BootstrapDelivery::Pending,
    }
}
pub fn manifest(runtime: &RuntimeRecord) -> TerminalManifest {
    TerminalManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        name: NAME.into(),
        command: runtime.command.clone(),
        args: runtime.args.clone(),
        cwd: runtime.cwd.clone(),
        cols: runtime.cols,
        rows: runtime.rows,
        created_at_ms: CLOCK,
        updated_at_ms: CLOCK,
        archived: false,
        recovery_state: RecoveryState::Created,
    }
}
pub fn checkpoint(launch: &WorkerLaunchSpec, runtime: &RuntimeRecord) -> StartupCheckpointEvidence {
    StartupCheckpointEvidence {
        schema_version: STARTUP_CHECKPOINT_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        controller_generation: runtime.controller_generation,
        launch_sha256: StartupLaunchFence::from_launch(launch)
            .unwrap()
            .launch_sha256,
        boot_id: runtime.boot_id.clone(),
        worker: StartupProcessIdentity::new(runtime.worker_pid, runtime.worker_start_time),
        child: Some(StartupProcessIdentity::new(
            runtime.child_pid,
            runtime.child_start_time,
        )),
        stage: StartupStage::RuntimeActivation,
        phase: StartupPhase::Entered,
        status: StartupStatus::Active,
        successor: None,
        predecessor_runtime_sha256: None,
        pending_pty_custody_rollover: None,
    }
}
