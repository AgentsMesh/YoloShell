use startup_checkpoint::StartupStage as Stage;

pub(crate) const BOOTSTRAP_SURVIVOR_LOG: &str = "serving bootstrap survivor runtime";

pub(crate) struct PreparedWorkerStartup {
    launch: WorkerLaunchSpec,
    agent_monitor: Option<AgentMonitorConfig>,
    worker_identity: process::ProcessIdentity,
    checkpoint: startup_checkpoint::ActiveStartupCheckpoint,
}

pub(crate) const STARTUP_CHILD_IDENTITY_CONTEXT: &str = "cannot establish PTY child identity";
pub(crate) const STARTUP_CHILD_SPAWN_CONTEXT: &str = "spawn tracked PTY root";
pub(crate) const STARTUP_EXEC_REFRESH_CONTEXT: &str = "refresh startup process scope after exec";
pub(crate) const STARTUP_INPUT_REFRESH_CONTEXT: &str =
    "refresh startup process scope after bootstrap input";
pub(crate) const STARTUP_ADMISSION_LOCK_CONTEXT: &str =
    "session lifecycle mutation is in progress during worker startup";
pub(crate) const STARTUP_ADMISSION_MANIFEST_CONTEXT: &str = "read worker startup session manifest";
pub(crate) const STARTUP_ADMISSION_REJECTED: &str =
    "worker startup requires the current non-archived session manifest";
pub(crate) const STARTUP_PTY_PIXEL_SIZE: u16 = 0;

pub(crate) fn startup_pty_size(launch: &WorkerLaunchSpec) -> PtySize {
    PtySize {
        rows: launch.rows,
        cols: launch.cols,
        pixel_width: STARTUP_PTY_PIXEL_SIZE,
        pixel_height: STARTUP_PTY_PIXEL_SIZE,
    }
}

pub(crate) fn prepare_worker_startup(
    namespace: &WorkerNamespace,
    paths: &Paths,
    terminal_id: Uuid,
    provider_runtime: Arc<ProviderRuntimeRegistry>,
) -> Result<PreparedWorkerStartup> {
    let mut launch: WorkerLaunchSpec = namespace.read_json(&paths.worker_launch(terminal_id))?;
    anyhow::ensure!(
        launch.terminal_id == terminal_id,
        WORKER_LAUNCH_IDENTITY_ERROR
    );
    anyhow::ensure!(
        launch.schema_version == WORKER_LAUNCH_SCHEMA_VERSION,
        WORKER_LAUNCH_SCHEMA_ERROR
    );
    crate::agent::validate_launch_environment(&launch.environment)?;
    let agent_monitor = prepare_agent_monitor(paths, &launch, provider_runtime);
    validate_socket_path(&launch.socket_path)?;
    anyhow::ensure!(
        launch.socket_path == namespace.worker_socket(terminal_id)?,
        WORKER_SOCKET_NAMESPACE_MISMATCH_CONTEXT
    );
    let worker_identity =
        process::ProcessIdentity::capture(std::process::id()).context(WORKER_IDENTITY_CONTEXT)?;
    let _lifecycle_lock = paths
        .try_acquire_session_lifecycle_lock(terminal_id)?
        .context(STARTUP_ADMISSION_LOCK_CONTEXT)?;
    let manifest: crate::model::TerminalManifest = namespace
        .read_json(&paths.manifest(terminal_id))
        .context(STARTUP_ADMISSION_MANIFEST_CONTEXT)?;
    anyhow::ensure!(
        manifest.schema_version == crate::constants::SESSION_MANIFEST_SCHEMA_VERSION
            && manifest.terminal_id == terminal_id
            && !manifest.archived,
        STARTUP_ADMISSION_REJECTED
    );
    let checkpoint = startup_checkpoint::ActiveStartupCheckpoint::begin(
        namespace,
        paths,
        &launch,
        worker_identity,
    )?;
    launch.cols = normalize_terminal_columns(launch.cols);
    launch.rows = normalize_terminal_rows(launch.rows);
    Ok(PreparedWorkerStartup {
        launch,
        agent_monitor,
        worker_identity,
        checkpoint,
    })
}
