const RUNTIME_ERROR_PROCESS_HELPER_ENV: &str = "YOLO_RUNTIME_ERROR_PROCESS_HELPER";
const RUNTIME_ERROR_PROCESS_HELPER_ROLE: &str = "runtime-error-custody";
const RUNTIME_ERROR_PROCESS_HELPER_TEST: &str =
    "worker::tests::runtime_errors::concurrent_runtime_error_process_helper";
const RUNTIME_ERROR_PROCESS_FAILURE_DETAIL: &str = "concurrent runtime-error process failed";

fn launch_spec(paths: &Paths, terminal_id: Uuid) -> WorkerLaunchSpec {
    WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        socket_path: paths.worker_socket(terminal_id).unwrap(),
        auth_token: "token".into(),
        controller_generation: 1,
        command: "/definitely/missing/yolo-command".into(),
        args: Vec::new(),
        cwd: paths.home().to_path_buf(),
        environment: [
            (
                crate::agent::codex::constants::DATA_HOME_ENV.to_owned(),
                paths
                    .home()
                    .join(crate::agent::codex::constants::EXECUTABLE)
                    .display()
                    .to_string(),
            ),
            (
                crate::agent::claude::constants::DATA_HOME_ENV.to_owned(),
                paths
                    .home()
                    .join(crate::agent::claude::constants::EXECUTABLE)
                    .display()
                    .to_string(),
            ),
        ]
        .into(),
        initial_input: None,
        cols: 80,
        rows: 24,
        context: None,
    }
}

pub(super) fn write_launch(paths: &Paths, terminal_id: Uuid, launch: &WorkerLaunchSpec) {
    paths.ensure_session(terminal_id).unwrap();
    write_launch_manifest(paths, terminal_id, launch);
    atomic_write_json(&paths.worker_launch(terminal_id), launch).unwrap();
}

include!("unit/runtime/runtime_error_custody.rs");
