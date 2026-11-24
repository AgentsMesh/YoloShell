use crate::model::{BootstrapDelivery, RuntimeRecord};
use crate::paths::Paths;
use std::path::Path;
use uuid::Uuid;

const AUTH_TOKEN: &str = "typed-reboot-auth-token";
pub(super) const CHILD_PID: u32 = 73;
const CHILD_START: u64 = 74;
const COLUMNS: u16 = 80;
const GENERATION: u64 = 1;
const LAUNCH_DIGEST_BYTE: u8 = 1;
const ROWS: u16 = 24;
const SESSION_NAME: &str = "durable reboot validation";
const SOCKET_FILE: &str = "yolo-reboot-typed.sock";
const STARTED_AT_MS: i64 = 1;
pub(super) const WORKER_PID: u32 = 71;
pub(super) const WORKER_START: u64 = 72;

pub(super) fn write_manifest(paths: &Paths, runtime: &RuntimeRecord) {
    let manifest = crate::model::TerminalManifest {
        schema_version: crate::constants::SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        name: SESSION_NAME.to_owned(),
        command: runtime.command.clone(),
        args: runtime.args.clone(),
        cwd: runtime.cwd.clone(),
        cols: runtime.cols,
        rows: runtime.rows,
        created_at_ms: runtime.started_at_ms,
        updated_at_ms: runtime.started_at_ms,
        archived: false,
        recovery_state: crate::model::RecoveryState::Created,
    };
    crate::paths::atomic_write_json(&paths.manifest(runtime.terminal_id), &manifest).unwrap();
}

pub(super) fn runtime(boot_id: &str, fixture_root: &Path) -> RuntimeRecord {
    RuntimeRecord {
        schema_version: crate::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: WORKER_PID,
        worker_start_time: WORKER_START,
        child_pid: CHILD_PID,
        child_start_time: CHILD_START,
        boot_id: boot_id.to_owned(),
        socket_path: fixture_root.join(SOCKET_FILE),
        auth_token: AUTH_TOKEN.to_owned(),
        controller_generation: GENERATION,
        started_at_ms: STARTED_AT_MS,
        command: yolo_agent_core::constants::FALLBACK_SHELL.to_owned(),
        args: Vec::new(),
        cwd: fixture_root.to_owned(),
        cols: COLUMNS,
        rows: ROWS,
        bootstrap_delivery: BootstrapDelivery::Delivered,
    }
}

pub(super) fn checkpoint(runtime: &RuntimeRecord) -> crate::worker::StartupCheckpointEvidence {
    crate::worker::StartupCheckpointEvidence {
        schema_version: crate::worker::constants::STARTUP_CHECKPOINT_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        controller_generation: runtime.controller_generation,
        launch_sha256: crate::sha256_digest::Sha256Digest::repeated(LAUNCH_DIGEST_BYTE),
        boot_id: runtime.boot_id.clone(),
        worker: crate::worker::StartupProcessIdentity {
            pid: WORKER_PID,
            start_time: WORKER_START,
        },
        child: Some(crate::worker::StartupProcessIdentity {
            pid: CHILD_PID,
            start_time: CHILD_START,
        }),
        stage: crate::worker::StartupStage::RuntimeActivation,
        phase: crate::worker::StartupPhase::Completed,
        status: crate::worker::StartupStatus::RuntimeActivated,
        successor: None,
        predecessor_runtime_sha256: None,
        pending_pty_custody_rollover: None,
    }
}
