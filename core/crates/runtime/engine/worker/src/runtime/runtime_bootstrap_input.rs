use super::claude_hook::{self, ClaudeHookAdmissionPump, ClaudeHookServer};
use super::runtime_provider_hook::ProviderHookServer;
use super::{
    BootstrapAttempt, BootstrapDelivery, Journal, MasterPty, Paths, PtyInputWriter, RuntimeRecord,
    StartupPtyResources, WorkerLaunchSpec, WorkerNamespace, WorkerSecondaryCustodySource,
    WorkerState, deliver_bootstrap_input, lock_recover, prepare_worker_runtime, process,
    startup_checkpoint, supervision,
};
use anyhow::{Context, Result};
use std::io::Read;
use std::sync::{Arc, Mutex};
use tokio::task::JoinSet;
use uuid::Uuid;
use yolo_agent_providers::ProviderRuntimeRegistry;

pub(crate) struct ActivationInput<'a> {
    pub(crate) paths: Paths,
    pub(crate) namespace: WorkerNamespace,
    pub(crate) record: RuntimeRecord,
    pub(crate) reader: Box<dyn Read + Send>,
    pub(crate) writer: PtyInputWriter,
    pub(crate) journal: Arc<Mutex<Journal>>,
    pub(crate) launch: &'a WorkerLaunchSpec,
    pub(crate) pty_session_capability: Uuid,
    pub(crate) secondary_custody: Arc<WorkerSecondaryCustodySource>,
    pub(crate) claude_hook: ClaudeHookServer,
    pub(crate) provider_hook: ProviderHookServer,
    pub(crate) codex_hook_ingress: super::codex_hook::CodexHookIngressSupervisor,
    pub(crate) provider_runtime: Arc<ProviderRuntimeRegistry>,
    pub(crate) shell: process::ProcessIdentity,
}

pub(crate) struct ActivatedRuntime {
    pub(crate) state: Arc<WorkerState>,
    pub(crate) supervised_tasks: JoinSet<Option<supervision::TaskFailure>>,
}

pub(crate) async fn activate(
    input: ActivationInput<'_>,
    startup: &mut StartupPtyResources<Box<dyn MasterPty + Send>>,
    checkpoint: &mut startup_checkpoint::ActiveStartupCheckpoint,
) -> Result<ActivatedRuntime> {
    checkpoint.enter(
        &input.namespace,
        &input.paths,
        startup_checkpoint::StartupStage::BootstrapInput,
    )?;
    let mut prepared = prepare_worker_runtime(
        input.paths,
        input.namespace,
        input.record,
        startup,
        input.reader,
        input.writer,
        input.journal,
        input.launch,
        input.pty_session_capability,
        input.secondary_custody,
        input.codex_hook_ingress,
        input.provider_runtime,
    )?;
    let prepared_state = Arc::clone(prepared.state());
    let mut supervised_tasks = JoinSet::new();
    super::codex_hook::spawn_projection(&mut supervised_tasks, Arc::clone(&prepared_state))?;
    let mut hook_pump = claude_hook::spawn_admission_pump(
        &mut supervised_tasks,
        input.claude_hook,
        Arc::clone(&prepared_state),
        input.shell,
    );
    let mut provider_pump = super::runtime_provider_hook::start(
        &mut supervised_tasks,
        input.provider_hook,
        Arc::clone(&prepared_state),
        input.shell,
        input.launch.initial_input.is_some(),
    )
    .await?;
    let bootstrap = deliver_ready_input(
        &mut supervised_tasks,
        &mut hook_pump,
        prepared.writer_mut(),
        input.launch.initial_input.as_deref(),
    )
    .await?;
    startup
        .child_mut()
        .refresh()
        .context(super::STARTUP_INPUT_REFRESH_CONTEXT)?;
    checkpoint.complete(
        &prepared_state.namespace,
        &prepared_state.paths,
        startup_checkpoint::StartupStage::BootstrapInput,
    )?;
    let survivor_detail = settle_attempt(&prepared_state, bootstrap);
    checkpoint.enter(
        &prepared_state.namespace,
        &prepared_state.paths,
        startup_checkpoint::StartupStage::RuntimeActivation,
    )?;
    let staged = prepared.stage(startup)?;
    let state = staged.publish_and_activate(startup, checkpoint)?;
    hook_pump.activate()?;
    provider_pump.activate()?;
    if let Some(detail) = survivor_detail {
        let terminal_id = state.terminal_id();
        tracing::warn!(%terminal_id, %detail, "{}", super::BOOTSTRAP_SURVIVOR_LOG);
    }
    Ok(ActivatedRuntime {
        state,
        supervised_tasks,
    })
}

pub(crate) async fn deliver_ready_input(
    tasks: &mut JoinSet<Option<supervision::TaskFailure>>,
    pump: &mut ClaudeHookAdmissionPump,
    writer: &mut PtyInputWriter,
    input: Option<&[u8]>,
) -> Result<BootstrapAttempt> {
    let Some(input) = input else {
        pump.retain_readiness(tasks)?;
        return Ok(deliver_bootstrap_input(writer, None));
    };
    pump.wait_ready().await?;
    Ok(deliver_bootstrap_input(writer, Some(input)))
}

pub(crate) fn settle_attempt(state: &WorkerState, attempt: BootstrapAttempt) -> Option<String> {
    let detail = attempt.error.map(|error| error.to_string());
    let mut record = lock_recover(&state.record);
    record.bootstrap_delivery = attempt.delivery;
    if !record.bootstrap_delivery.is_ready() {
        record.bootstrap_delivery = BootstrapDelivery::Survivor;
    }
    detail
}
