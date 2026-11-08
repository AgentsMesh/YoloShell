use yolo_agent_providers::ProviderRuntimeRegistry;
#[derive(Clone)]
pub(crate) struct AgentMonitorConfig {
    pub(crate) context: DetectionContext,
    pub(crate) binding_path: PathBuf,
    pub(crate) registry: Arc<AgentRegistry>,
    pub(crate) provider_runtime: Arc<ProviderRuntimeRegistry>,
}

pub(crate) fn prepare_agent_monitor(
    paths: &Paths,
    launch: &WorkerLaunchSpec,
    provider_runtime: Arc<ProviderRuntimeRegistry>,
) -> Option<AgentMonitorConfig> {
    let registry = Arc::new(crate::agent::agent_registry_from_runtime(&provider_runtime));
    let context = registry.detection_context(
        launch.terminal_id,
        launch.cwd.clone(),
        &launch.environment,
    )?;
    Some(AgentMonitorConfig {
        context,
        binding_path: paths.agent_binding(launch.terminal_id),
        registry,
        provider_runtime,
    })
}

fn build_agent_monitor_retained(
    state: &WorkerState,
    config: &AgentMonitorConfig,
) -> Result<(AgentDetector, AgentBindingStore)> {
    let store = AgentBindingStore::with_provider_runtime(
        config.binding_path.clone(),
        Arc::clone(&config.provider_runtime),
    );
    let binding = store
        .load_authoritative_in_namespace(
            state.namespace.lease(),
            &config.context,
            claude_hook::binding::runtime_identity(state),
        )
        .context("load agent monitor binding")?;
    let detector = AgentDetector::new(
        config.context.clone(),
        AgentRegistry::clone(&config.registry),
        binding,
    )?;
    Ok((detector, store))
}

#[cfg(test)]
pub(crate) fn build_agent_monitor(
    config: &AgentMonitorConfig,
) -> Result<(AgentDetector, AgentBindingStore)> {
    let store = AgentBindingStore::with_provider_runtime(
        config.binding_path.clone(),
        Arc::clone(&config.provider_runtime),
    );
    let binding = store
        .load(&config.context)
        .context("load agent monitor binding")?;
    let detector = AgentDetector::new(
        config.context.clone(),
        AgentRegistry::clone(&config.registry),
        binding,
    )?;
    Ok((detector, store))
}

pub(crate) async fn supervise_agent_monitor(
    state: Arc<WorkerState>,
    config: AgentMonitorConfig,
    shell_pid: u32,
) {
    let operation_state = Arc::clone(&state);
    let final_scan_persisted = Arc::new(AtomicBool::new(false));
    let operation_complete = Arc::clone(&final_scan_persisted);
    let stop_complete = Arc::clone(&final_scan_persisted);
    supervision::restart_task(
        supervision::AGENT_MONITOR_TASK,
        supervision::RestartPolicy::default(),
        move || {
            let state = Arc::clone(&operation_state);
            let config = config.clone();
            let completion = Arc::clone(&operation_complete);
            async move {
                let (detector, store) = {
                    let _gate = state.agent_generation_gate.lock().await;
                    build_agent_monitor_retained(&state, &config)?
                };
                monitor_agent(state, config.context, detector, store, shell_pid).await?;
                completion.store(true, Ordering::Release);
                Ok(())
            }
        },
        move || stop_complete.load(Ordering::Acquire),
    )
    .await;
}

pub(crate) fn spawn_agent_monitor_task(
    tasks: &mut JoinSet<Option<supervision::TaskFailure>>,
    state: Arc<WorkerState>,
    config: AgentMonitorConfig,
    shell_pid: u32,
) {
    supervision::spawn_task(tasks, supervision::AGENT_MONITOR_TASK, async move {
        supervise_agent_monitor(state, config, shell_pid).await;
        Ok(())
    });
}

async fn monitor_agent(
    state: Arc<WorkerState>,
    context: DetectionContext,
    mut detector: AgentDetector,
    store: AgentBindingStore,
    shell_pid: u32,
) -> Result<()> {
    let mut interval = tokio::time::interval(crate::agent::constants::DETECTION_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    refresh_agent_title(&state, detector.binding().cloned()).await?;
    loop {
        interval.tick().await;
        let ended = state.child_waited.load(Ordering::Acquire);
        let binding = {
            let _gate = state.agent_generation_gate.lock().await;
            if ended {
                // Retain the last provider title: it remains the most useful
                // historical label for an ended agent run.
                persist_agent_detection_locked(&state, &store, detector.session_exited())?;
                return Ok(());
            }
            let processes = scan_agent_processes(&state, shell_pid).await?;
            let runtime = claude_hook::binding::runtime_identity(&state);
            let repaired = synchronize_native_projection(&state, &store, &context, runtime)?;
            if let Some(binding) = repaired {
                publish_agent_detection(&state, detector.synchronize_binding(binding));
            }
            let result = detect_scan(&mut detector, Ok(&processes)).map_err(|error| {
                tracing::warn!(
                    error_code = error.code(),
                    %error,
                    "agent detection scan failed"
                );
                anyhow::Error::from(error)
            })?;
            if result != DetectionResult::Unchanged
                && scan_agent_processes(&state, shell_pid).await? != processes
            {
                continue;
            }
            persist_agent_detection_locked(&state, &store, result)?;
            if let Some(binding) =
                store.load_authoritative_in_namespace(state.namespace.lease(), &context, runtime)?
            {
                detector.synchronize_binding(binding);
            }
            detector.binding().cloned()
        };
        refresh_agent_title(&state, binding).await?;
    }
}

include!("agent_monitor_publication.rs");
include!("agent_monitor_native.rs");
