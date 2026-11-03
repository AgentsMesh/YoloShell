use crate::wire_fixture_constants::*;
use anyhow::Result;
use serde::Serialize;
use uuid::Uuid;
use yolo_application_support::constants::{RUNTIME_RECORD_SCHEMA_VERSION, WORKER_LAUNCH_SCHEMA_VERSION};
use yolo_crypto::Sha256Digest;
use yolo_startup_checkpoint_contract::{StartupLaunchFence, constants::PREDECESSOR_RUNTIME_DIGEST_DOMAIN};
use yolo_terminal_model::{BootstrapDelivery, RuntimeRecord, WorkerLaunchSpec, DEFAULT_COLS, DEFAULT_ROWS};

#[derive(Serialize)]
struct DigestFixture {
    launch_json: String,
    launch_sha256: Sha256Digest,
    runtime_json: String,
    runtime_sha256: Sha256Digest,
}

pub fn emit() -> Result<()> {
    let launch = WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: Uuid::from_u128(TERMINAL),
        runtime_epoch: Uuid::from_u128(EPOCH),
        worker_id: Uuid::from_u128(WORKER),
        socket_path: SAMPLE_SOCKET.into(),
        auth_token: SAMPLE_AUTH.into(),
        controller_generation: GENERATION,
        command: SAMPLE_EXECUTABLE.into(),
        args: vec![SAMPLE_ARGUMENT.into()],
        cwd: SAMPLE_DIRECTORY.into(),
        environment: [(ENVIRONMENT_KEY.into(), SAMPLE_ARGUMENT.into())].into(),
        initial_input: Some(SAMPLE_ARGUMENT.as_bytes().to_vec()),
        cols: DEFAULT_COLS,
        rows: DEFAULT_ROWS,
        context: None,
    };
    let runtime = RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: launch.terminal_id,
        runtime_epoch: launch.runtime_epoch,
        worker_id: launch.worker_id,
        worker_pid: WORKER_PID,
        worker_start_time: WORKER_START,
        child_pid: CHILD_PID,
        child_start_time: CHILD_START,
        boot_id: BOOT.into(),
        socket_path: launch.socket_path.clone(),
        auth_token: launch.auth_token.clone(),
        controller_generation: launch.controller_generation,
        started_at_ms: STARTED_AT_MS,
        command: launch.command.clone(),
        args: launch.args.clone(),
        cwd: launch.cwd.clone(),
        cols: launch.cols,
        rows: launch.rows,
        bootstrap_delivery: BootstrapDelivery::Pending,
    };
    let fixture = DigestFixture {
        launch_json: serde_json::to_string_pretty(&launch)?,
        launch_sha256: StartupLaunchFence::from_launch(&launch)?.launch_sha256,
        runtime_json: serde_json::to_string_pretty(&runtime)?,
        runtime_sha256: Sha256Digest::capture(&(PREDECESSOR_RUNTIME_DIGEST_DOMAIN, runtime))?,
    };
    serde_json::to_writer(std::io::stdout().lock(), &fixture)?;
    Ok(())
}
