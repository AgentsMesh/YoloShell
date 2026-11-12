const SURVIVOR_STARTUP_DIAGNOSTIC_UNAVAILABLE: &str = "unavailable";

fn survivor_startup_diagnostic(paths: &Paths, terminal_id: Uuid) -> String {
    let checkpoint = read_json::<startup_checkpoint::StartupCheckpointEvidence>(
        &paths.worker_startup_checkpoint(terminal_id),
    )
    .map(|value| format!("{:?}/{:?}/{:?}", value.stage, value.phase, value.status))
    .unwrap_or_else(|error| error.to_string());
    let runtime = read_json::<RuntimeRecord>(&paths.runtime_record(terminal_id))
        .map(|value| format!("{:?}", value.bootstrap_delivery))
        .unwrap_or_else(|error| error.to_string());
    let failure = runtime_result_test_hook::take(terminal_id)
        .unwrap_or_else(|| SURVIVOR_STARTUP_DIAGNOSTIC_UNAVAILABLE.to_owned());
    format!("checkpoint={checkpoint}; runtime={runtime}; pre-cleanup={failure}")
}
