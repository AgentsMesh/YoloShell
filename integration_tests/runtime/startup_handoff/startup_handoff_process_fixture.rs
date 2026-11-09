use super::watchdog::WorkerWatchdog;
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;
use uuid::Uuid;
use yolo_runtime::cli::constants::{HOME_ARGUMENT, TERMINAL_ID_ARGUMENT, SESSION_WORKER_COMMAND};
use yolo_runtime::constants::{SESSION_MANIFEST_SCHEMA_VERSION, WORKER_LAUNCH_SCHEMA_VERSION};
use yolo_runtime::model::{RecoveryState, TerminalManifest, WorkerLaunchSpec, now_ms};
use yolo_runtime::paths::{Paths, atomic_write_json};
use yolo_runtime::worker::startup_handoff_test_hook::{
    FAILURE_MARKER_FILE, SPEC_ENVIRONMENT, StartupHandoffTestSpec,
};

const HOST_BINARY_ENVIRONMENT: &str = "YOLO_TEST_HOST_BINARY";
const TEST_AUTH_TOKEN: &str = "startup-handoff-process-token";
const TEST_SESSION_NAME: &str = "startup handoff process test";
const TEST_COMMAND: &str = "/bin/sleep";
const TEST_CHILD_SLEEP_SECONDS: u64 = 30;
const TEST_CONTROLLER_GENERATION: u64 = 1;
const TEST_COLUMNS: u16 = 80;
const TEST_ROWS: u16 = 24;
const HOST_BINARY_ENVIRONMENT_CONTEXT: &str = "locate adversarial Yolo host binary";
const HOST_BINARY_RESOLUTION_CONTEXT: &str = "resolve adversarial Yolo host binary";
const HOST_BINARY_REGULAR_DETAIL: &str = "resolved adversarial Yolo host binary is not regular";

pub(super) struct Fixture {
    _home: TempDir,
    pub(super) paths: Paths,
    pub(super) terminal_id: Uuid,
    spec: StartupHandoffTestSpec,
}

impl Fixture {
    pub(super) fn new() -> Result<Self> {
        let home = tempfile::tempdir()?;
        let paths = Paths::new(home.path());
        let terminal_id = Uuid::new_v4();
        paths.ensure_session(terminal_id)?;
        let spec = StartupHandoffTestSpec {
            run_id: Uuid::new_v4(),
            directory: paths.events_dir(terminal_id),
        };
        write_session(&paths, terminal_id)?;
        Ok(Self {
            _home: home,
            paths,
            terminal_id,
            spec,
        })
    }

    pub(super) fn marker(&self, file: &str) -> PathBuf {
        self.spec.marker(file)
    }

    pub(super) fn spawn_worker(&self) -> Result<WorkerWatchdog> {
        let executable = host_binary()?;
        let child = Command::new(executable)
            .args(yolo_launch_contract::ProcessRole::Host.arguments())
            .arg(SESSION_WORKER_COMMAND)
            .arg(HOME_ARGUMENT)
            .arg(self.paths.home())
            .arg(TERMINAL_ID_ARGUMENT)
            .arg(self.terminal_id.to_string())
            .env(SPEC_ENVIRONMENT, serde_json::to_string(&self.spec)?)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        WorkerWatchdog::new(child, self.marker(FAILURE_MARKER_FILE))
    }
}

fn host_binary() -> Result<PathBuf> {
    let path = std::env::var_os(HOST_BINARY_ENVIRONMENT)
        .map(PathBuf::from)
        .context(HOST_BINARY_ENVIRONMENT_CONTEXT)?;
    resolve_host_binary(&path)
}

fn resolve_host_binary(path: &Path) -> Result<PathBuf> {
    let resolved = path
        .canonicalize()
        .context(HOST_BINARY_RESOLUTION_CONTEXT)?;
    ensure!(
        resolved.symlink_metadata()?.file_type().is_file(),
        HOST_BINARY_REGULAR_DETAIL
    );
    Ok(resolved)
}

fn write_session(paths: &Paths, terminal_id: Uuid) -> Result<()> {
    let timestamp = now_ms();
    let command_args = vec![TEST_CHILD_SLEEP_SECONDS.to_string()];
    let launch = WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        socket_path: paths.worker_socket(terminal_id)?,
        auth_token: TEST_AUTH_TOKEN.to_owned(),
        controller_generation: TEST_CONTROLLER_GENERATION,
        command: TEST_COMMAND.to_owned(),
        args: command_args.clone(),
        cwd: paths.home().to_owned(),
        environment: Default::default(),
        initial_input: None,
        cols: TEST_COLUMNS,
        rows: TEST_ROWS,
    };
    let manifest = TerminalManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id,
        name: TEST_SESSION_NAME.to_owned(),
        command: TEST_COMMAND.to_owned(),
        args: command_args,
        cwd: paths.home().to_owned(),
        cols: TEST_COLUMNS,
        rows: TEST_ROWS,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        archived: false,
        recovery_state: RecoveryState::Created,
    };
    atomic_write_json(&paths.manifest(terminal_id), &manifest)?;
    atomic_write_json(&paths.worker_launch(terminal_id), &launch)
}

#[cfg(test)]
#[path = "startup_handoff_process_fixture_tests.rs"]
mod tests;
