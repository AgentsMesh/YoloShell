const TEST_AUTH_TOKEN: &str = "startup-adversarial-token";
const TEST_SESSION_NAME: &str = "startup adversarial session";
const TEST_SHELL: &str = "/bin/sh";
const TEST_SHELL_COMMAND_FLAG: &str = "-c";
const TEST_SHELL_ARG_ZERO: &str = "yolo-worker-test";
const TEST_MARKER_SCRIPT: &str = "printf x >> \"$1\"; exec /bin/sleep \"$2\"";
const TEST_MARKER_CONTENT: &[u8] = b"x";
const TEST_CHILD_SLEEP_SECONDS: u64 = 30;
const TEST_COLUMNS: u16 = 80;
const TEST_ROWS: u16 = 24;
const TEST_CONTROLLER_GENERATION: u64 = 1;
const TEST_READY_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TEST_DUPLICATE_OBSERVATION_DELAY: Duration = Duration::from_millis(50);
const TEST_BACKLOG_CONNECT_TIMEOUT: Duration = Duration::from_millis(50);
const TEST_BACKLOG_ATTEMPTS: usize = 16;
const TEST_LISTEN_BACKLOG: libc::c_int = 1;
include!("unit/startup/startup_adversarial_support.rs");

struct TestProcessCleanup(Option<process::ProcessIdentity>);

impl Drop for TestProcessCleanup {
    fn drop(&mut self) {
        if let Some(identity) = self.0 {
            let _ = identity.signal(libc::SIGKILL);
        }
    }
}

fn adversarial_launch(paths: &Paths, terminal_id: Uuid, marker: &Path) -> WorkerLaunchSpec {
    WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        socket_path: paths.worker_socket(terminal_id).unwrap(),
        auth_token: TEST_AUTH_TOKEN.into(),
        controller_generation: TEST_CONTROLLER_GENERATION,
        command: TEST_SHELL.into(),
        args: vec![
            TEST_SHELL_COMMAND_FLAG.into(),
            TEST_MARKER_SCRIPT.into(),
            TEST_SHELL_ARG_ZERO.into(),
            marker.to_string_lossy().into_owned(),
            TEST_CHILD_SLEEP_SECONDS.to_string(),
        ],
        cwd: paths.home().to_owned(),
        environment: Default::default(),
        initial_input: None,
        cols: TEST_COLUMNS,
        rows: TEST_ROWS,
        context: None,
    }
}

fn write_adversarial_manifest(paths: &Paths, terminal_id: Uuid) {
    let timestamp = now_ms();
    let manifest = crate::model::TerminalManifest {
        schema_version: crate::constants::SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: terminal_id,
        name: TEST_SESSION_NAME.into(),
        command: TEST_SHELL.into(),
        args: Vec::new(),
        cwd: paths.home().to_owned(),
        cols: TEST_COLUMNS,
        rows: TEST_ROWS,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        archived: false,
        recovery_state: crate::model::RecoveryState::Created,
    };
    atomic_write_json(&paths.manifest(terminal_id), &manifest).unwrap();
}
