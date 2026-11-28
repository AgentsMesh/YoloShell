fn runtime_record(
    paths: &Paths,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    child_pid: u32,
) -> RuntimeRecord {
    RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: terminal_id,
        runtime_epoch,
        worker_id: Uuid::new_v4(),
        worker_pid: std::process::id(),
        worker_start_time: process::process_start_time(std::process::id())
            .unwrap_or(FIXTURE_UNKNOWN_PROCESS_START),
        child_pid,
        child_start_time: process::process_start_time(child_pid).unwrap(),
        boot_id: process::boot_id(),
        socket_path: paths.worker_socket(terminal_id).unwrap(),
        auth_token: FIXTURE_AUTH_TOKEN.into(),
        controller_generation: FIXTURE_CONTROLLER_GENERATION,
        started_at_ms: now_ms(),
        command: FIXTURE_COMMAND.into(),
        args: Vec::new(),
        cwd: paths.home().to_path_buf(),
        cols: FIXTURE_TERMINAL_COLUMNS,
        rows: FIXTURE_TERMINAL_ROWS,
        bootstrap_delivery: BootstrapDelivery::NotRequired,
    }
}
