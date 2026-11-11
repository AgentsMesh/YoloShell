pub(in crate::host) fn classify_creation_launch(
    launch: Option<&WorkerLaunchSpec>,
    binding: &CreationBinding,
    planned: &CreationLaunchIdentity,
    paths: &Paths,
) -> CreationEvidenceIdentity {
    let Some(launch) = launch else {
        return CreationEvidenceIdentity::Missing;
    };
    if launch_matches_binding(launch, binding, planned, paths) {
        CreationEvidenceIdentity::Current
    } else if creation_retry_fences_previous(binding, planned)
        && launch_matches_product(launch, binding, paths)
        && !launch_matches_identity(launch, planned)
    {
        CreationEvidenceIdentity::Previous
    } else {
        CreationEvidenceIdentity::Invalid
    }
}

pub(crate) fn classify_creation_runtime(
    runtime: Option<&RuntimeRecord>,
    launch: Option<&WorkerLaunchSpec>,
    binding: &CreationBinding,
    planned: &CreationLaunchIdentity,
    paths: &Paths,
) -> CreationEvidenceIdentity {
    let Some(runtime) = runtime else {
        return CreationEvidenceIdentity::Missing;
    };
    if runtime_matches_binding(runtime, launch, binding, planned, paths) {
        CreationEvidenceIdentity::Current
    } else if creation_retry_fences_previous(binding, planned)
        && runtime_matches_product(runtime, binding, paths)
        && !runtime_matches_identity(runtime, planned)
    {
        CreationEvidenceIdentity::Previous
    } else {
        CreationEvidenceIdentity::Invalid
    }
}

pub(crate) fn creation_retry_fences_previous(
    binding: &CreationBinding,
    planned: &CreationLaunchIdentity,
) -> bool {
    matches!(
        binding.phase,
        CreationPhase::Launching | CreationPhase::SpawnFailed
    ) && binding.launch_generation > INITIAL_CREATION_LAUNCH_GENERATION
        && binding.launch.as_ref() == Some(planned)
}

pub(crate) fn launch_matches_binding(
    launch: &WorkerLaunchSpec,
    binding: &CreationBinding,
    planned: &CreationLaunchIdentity,
    paths: &Paths,
) -> bool {
    launch_matches_identity(launch, planned) && launch_matches_product(launch, binding, paths)
}

pub(crate) fn launch_matches_identity(
    launch: &WorkerLaunchSpec,
    planned: &CreationLaunchIdentity,
) -> bool {
    launch.runtime_epoch == planned.runtime_epoch
        && launch.worker_id == planned.worker_id
        && launch.auth_token == planned.auth_token
}

pub(crate) fn launch_matches_product(
    launch: &WorkerLaunchSpec,
    binding: &CreationBinding,
    paths: &Paths,
) -> bool {
    let shell = shell_command(&binding.spec.cwd);
    let expected_socket = paths.worker_socket(binding.result.terminal_id).ok();
    launch.schema_version == WORKER_LAUNCH_SCHEMA_VERSION
        && launch.terminal_id == binding.result.terminal_id
        && !launch.runtime_epoch.is_nil()
        && !launch.worker_id.is_nil()
        && valid_lower_hex(&launch.auth_token, WORKER_AUTH_TOKEN_HEX_LENGTH)
        && expected_socket.as_ref() == Some(&launch.socket_path)
        && launch.controller_generation != 0
        && launch.command == shell.executable.display().to_string()
        && launch.args == shell.args
        && launch.cwd == binding.spec.cwd
        && normalize_terminal_columns(launch.cols)
            == normalize_terminal_columns(binding.spec.cols.unwrap_or(DEFAULT_COLS))
        && normalize_terminal_rows(launch.rows)
            == normalize_terminal_rows(binding.spec.rows.unwrap_or(DEFAULT_ROWS))
        && launch.environment.is_empty()
        && launch.context == binding.spec.context
        && launch.initial_input == binding.spec.initial_input
}

pub(crate) fn runtime_matches_binding(
    record: &RuntimeRecord,
    launch: Option<&WorkerLaunchSpec>,
    binding: &CreationBinding,
    planned: &CreationLaunchIdentity,
    paths: &Paths,
) -> bool {
    runtime_matches_identity(record, planned)
        && runtime_matches_product(record, binding, paths)
        && launch.is_none_or(|value| value.controller_generation <= record.controller_generation)
}

pub(crate) fn runtime_matches_identity(
    record: &RuntimeRecord,
    planned: &CreationLaunchIdentity,
) -> bool {
    record.runtime_epoch == planned.runtime_epoch
        && record.worker_id == planned.worker_id
        && record.auth_token == planned.auth_token
}

pub(crate) fn runtime_matches_product(
    record: &RuntimeRecord,
    binding: &CreationBinding,
    paths: &Paths,
) -> bool {
    let shell = shell_command(&binding.spec.cwd);
    let expected_socket = paths.worker_socket(binding.result.terminal_id).ok();
    record.schema_version == RUNTIME_RECORD_SCHEMA_VERSION
        && record.terminal_id == binding.result.terminal_id
        && !record.runtime_epoch.is_nil()
        && !record.worker_id.is_nil()
        && valid_lower_hex(&record.auth_token, WORKER_AUTH_TOKEN_HEX_LENGTH)
        && expected_socket.as_ref() == Some(&record.socket_path)
        && record.controller_generation != 0
        && !record.boot_id.is_empty()
        && record.started_at_ms > 0
        && record.command == shell.executable.display().to_string()
        && record.args == shell.args
        && record.cwd == binding.spec.cwd
        // The original geometry remains fenced in the manifest and launch.
        // A live writer may resize the runtime before creation is finalized.
        && record.cols == normalize_terminal_columns(record.cols)
        && record.rows == normalize_terminal_rows(record.rows)
        && valid_creation_process_identity(record.worker_pid, record.worker_start_time)
        && valid_creation_process_identity(record.child_pid, record.child_start_time)
        && record.worker_pid != record.child_pid
}

pub(crate) fn valid_creation_process_identity(pid: u32, start_time: u64) -> bool {
    pid != 0 && pid <= i32::MAX as u32 && start_time != 0
}
