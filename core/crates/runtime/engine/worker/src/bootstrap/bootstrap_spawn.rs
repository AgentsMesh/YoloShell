use super::runtime::runtime_provider_hook::ProviderHookEnvironment;
pub(crate) fn bootstrap_child_spec(launch: &WorkerLaunchSpec, token: Uuid) -> Result<BootstrapChildSpec> {
    Ok(BootstrapChildSpec {
        socket_path: launch.socket_path.clone(),
        terminal_id: launch.terminal_id,
        runtime_epoch: launch.runtime_epoch,
        worker_id: launch.worker_id,
        token,
        command: launch.command.clone(),
        args: launch.args.clone(),
    })
}

pub(crate) fn bootstrap_command(
    launch: &WorkerLaunchSpec,
    spec: &BootstrapChildSpec,
    claude_hook: Option<&claude_hook::ClaudeHookEnvironment>,
    provider_hook: Option<&ProviderHookEnvironment>,
    hook_context: Option<&NativeHookContext>,
) -> Result<BootstrapSpawnCommand> {
    let executable = std::env::current_exe().context("resolve bootstrap wrapper executable")?;
    let mut command = CommandBuilder::new(executable);
    configure_bootstrap_entry(&mut command);
    command.cwd(&launch.cwd);
    command.env(
        BOOTSTRAP_GATE_SPEC_ENV,
        bootstrap_spec_codec::encode(spec)?,
    );
    environment::configure(&mut command, &launch.environment);
    command.env(
        crate::agent::constants::YOLO_TERMINAL_ID_ENV,
        launch.terminal_id.to_string(),
    );
    if let Some(context) = launch.context {
        yolo_worker_context_env::apply(&mut command, context.hub_id, context.workspace_id, context.run_id);
    }
    command.env(
        crate::agent::constants::YOLO_RUNTIME_EPOCH_ENV,
        launch.runtime_epoch.to_string(),
    );
    command.env(
        crate::process::PTY_SESSION_CAPABILITY_ENV,
        spec.token.to_string(),
    );
    if let Some(claude_hook) = claude_hook {
        claude_hook.configure(&mut command)?;
    }
    if let Some(provider_hook) = provider_hook {
        provider_hook.configure(&mut command);
    }
    if let Some(hook_context) = hook_context {
        hook_context.configure(&mut command);
    }
    Ok(BootstrapSpawnCommand {
        command,
        gate: UnreleasedBootstrapGate,
    })
}

#[cfg(not(test))]
pub(crate) fn configure_bootstrap_entry(command: &mut CommandBuilder) {
    command.args(crate::cli::ProcessRole::Host.arguments());
    command.arg(crate::cli::constants::BOOTSTRAP_CHILD_COMMAND);
}

#[cfg(test)]
pub(crate) fn configure_bootstrap_entry(command: &mut CommandBuilder) {
    command.args([
        BOOTSTRAP_TEST_EXACT_ARGUMENT,
        BOOTSTRAP_TEST_ENTRY,
        BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
    ]);
}

pub(crate) fn pending_runtime_record(
    launch: &WorkerLaunchSpec,
    worker_pid: u32,
    worker_start_time: u64,
    child: process::ProcessIdentity,
) -> RuntimeRecord {
    RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: launch.terminal_id,
        runtime_epoch: launch.runtime_epoch,
        worker_id: launch.worker_id,
        worker_pid,
        worker_start_time,
        child_pid: child.pid,
        child_start_time: child.start_time,
        boot_id: process::boot_id(),
        socket_path: launch.socket_path.clone(),
        auth_token: launch.auth_token.clone(),
        controller_generation: launch.controller_generation,
        started_at_ms: now_ms(),
        command: launch.command.clone(),
        args: launch.args.clone(),
        cwd: launch.cwd.clone(),
        cols: launch.cols,
        rows: launch.rows,
        bootstrap_delivery: BootstrapDelivery::Pending,
    }
}
pub(crate) struct UnreleasedBootstrapGate;

pub(crate) struct BootstrapSpawnCommand {
    pub(crate) command: CommandBuilder,
    pub(crate) gate: UnreleasedBootstrapGate,
}

include!("native_hook_context.rs");
