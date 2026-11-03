#[path = "agent_monitor_publication_fence.rs"]
pub(crate) mod agent_monitor_publication_fence;

fn persist_agent_detection_locked(
    state: &WorkerState,
    store: &AgentBindingStore,
    result: DetectionResult,
) -> Result<()> {
    let Some((event_type, binding)) = event_binding(result) else {
        return Ok(());
    };
    let expected = lock_recover(&state.record).clone();
    let previous = store.read_raw_in_namespace(state.namespace.lease())?;
    let runtime = crate::agent::authority::AuthorityRuntimeIdentity {
        runtime_epoch: expected.runtime_epoch,
        worker: crate::agent::authority::AuthorityProcessIdentity {
            pid: expected.worker_pid,
            start_time: expected.worker_start_time,
        },
    };
    let publication_fence = agent_monitor_publication_fence::acquire(state, &expected)?;
    let binding = store
        .reconcile_detected_with_commit_fence(state.namespace.lease(), &binding, runtime, || {
            publication_fence.verify()
        })
        .with_context(|| {
            format!(
                "{}: persist agent binding",
                crate::agent::constants::error::BINDING_WRITE_FAILED
            )
        })?;
    if previous.as_ref() != Some(&binding) {
        state.publish_agent_event(event_type, binding);
    }
    Ok(())
}

fn publish_agent_detection(state: &WorkerState, result: DetectionResult) {
    let Some((event_type, binding)) = event_binding(result) else {
        return;
    };
    state.publish_agent_event(event_type, binding);
}

fn event_binding(result: DetectionResult) -> Option<(&'static str, AgentBinding)> {
    match result {
        DetectionResult::BindingChanged(binding) => {
            Some((crate::protocol::event::AGENT_BINDING_CHANGED, binding))
        }
        DetectionResult::StatusChanged(binding) => {
            Some((crate::protocol::event::AGENT_STATUS_CHANGED, binding))
        }
        DetectionResult::Unchanged => None,
    }
}

/// Provider titles come from evidence files owned by the agent CLI. Resolve
/// them on the blocking pool: the terminal reader and input writer share this
/// runtime, so a slow home directory must not stall PTY echo.
pub(crate) async fn refresh_agent_title(
    state: &WorkerState,
    binding: Option<AgentBinding>,
) -> Result<()> {
    let Some(binding) = binding.filter(|binding| binding.state != crate::agent::AgentState::Exited)
    else {
        // The shell outlives its AgentRun. Keep the ended run's title only in
        // completed-state presentation; a live shell must fall back to the
        // PTY title instead of staying labelled by the conversation that ended.
        state.set_agent_title(None);
        return Ok(());
    };
    let registry = Arc::clone(&state.agent_registry);
    let title: Option<String> =
        tokio::task::spawn_blocking(move || {
            binding_display_title_with_registry(&binding, &registry)
        })
            .await
            .context("join agent display title read")?;
    state.set_agent_title(title);
    Ok(())
}

async fn scan_agent_processes(
    state: &WorkerState,
    shell_pid: u32,
) -> Result<Vec<crate::agent::ObservedProcess>> {
    let source = crate::agent::RuntimeProcessSource::new(
        state.agent_registry.process_environment_keys(),
    );
    tokio::task::spawn_blocking(move || source.descendants(shell_pid))
        .await
        .context("join agent process scan")?
}

#[cfg(test)]
pub(crate) async fn persist_agent_detection(
    state: &WorkerState,
    store: &AgentBindingStore,
    result: DetectionResult,
) -> Result<()> {
    let _gate = state.agent_generation_gate.lock().await;
    persist_agent_detection_locked(state, store, result)
}

