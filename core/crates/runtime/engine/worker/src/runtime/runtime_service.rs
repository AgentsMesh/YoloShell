pub(crate) async fn serve_worker_runtime(
    listener: UnixListener,
    state: Arc<WorkerState>,
    agent_monitor: Option<AgentMonitorConfig>,
    shell: process::ProcessIdentity,
    custody_lease: WorkerCustodyLease,
    mut supervised_tasks: JoinSet<Option<supervision::TaskFailure>>,
) -> Result<()> {
    let child_pid = shell.pid;
    let terminal_id = state.terminal_id();
    let worker_pid = std::process::id();
    let mut exit_events = state.events.subscribe();
    let mut custody_adoption_fence =
        secondary_custody_adoption_fence::CustodyAdoptionFence::armed(Arc::clone(&state));
    let custody_permit = state.admit_system_mutation().await;
    state
        .install_initial_ready_custody(&custody_permit, &custody_lease)
        .map_err(|error| anyhow::anyhow!(error.message))?;
    drop(custody_permit);
    let custody_identity = custody_lease.identity();
    let custody_lease = match secondary_custody_activation::activate(custody_lease).await {
        Ok(lease) => lease,
        Err(error) => {
            state.mark_custody_unavailable(custody_identity).await;
            return Err(error);
        }
    };
    custody_adoption_fence.disarm();
    state.runtime_serving.store(true, Ordering::Release);
    let custody_state = Arc::clone(&state);
    supervision::spawn_task(
        &mut supervised_tasks,
        supervision::PTY_CUSTODY_TASK,
        custody_lease.hold(custody_state),
    );
    if let Some(config) = agent_monitor.clone() {
        spawn_agent_monitor_task(&mut supervised_tasks, Arc::clone(&state), config, child_pid);
    }
    recover_deferred_natural_exit_shutdown(&state);
    tracing::info!(%terminal_id, worker_pid, child_pid, "session worker ready");
    let session_removal = wait_for_session_removal(Arc::clone(&state));
    tokio::pin!(session_removal);
    let mut accept_failures = 0_u32;
    let clients = Arc::new(Semaphore::new(MAX_WORKER_CLIENTS));
    let mut accept_retry = None;
    let (result, projection_shutdown) = loop {
        tokio::select! {
            accepted = listener.accept(), if accept_retry.is_none() => {
                accept_retry = handle_accept_result(
                    accepted.map(|(stream, address)| (UnixStream::from(stream), address)),
                    &mut supervised_tasks,
                    &state,
                    &clients,
                    &mut accept_failures,
                ).map(|delay| Box::pin(tokio::time::sleep(delay)));
            }
            _ = wait_for_accept_retry(&mut accept_retry) => accept_retry = None,
            _ = state.shutdown.notified() => {
                break (Ok(()), AgentProjectionShutdown::Invalidate)
            },
            completed = supervised_tasks.join_next(), if !supervised_tasks.is_empty() => {
                handle_task_completion(
                    completed,
                    &mut supervised_tasks,
                    &state,
                    agent_monitor.as_ref(),
                    child_pid,
                );
            }
            removal = &mut session_removal => {
                let removed_manifest = removal.removed_manifest();
                let result = match removal.terminate(&state, terminal_id).await {
                    Ok(SessionRemovalSettlement::RetryObservation) => {
                        session_removal.set(wait_for_session_removal(Arc::clone(&state)));
                        continue;
                    }
                    Ok(SessionRemovalSettlement::Terminated) => wait_for_orphan_exit_durability(
                        &state,
                        &mut exit_events,
                        WORKER_TERMINATION_TIMEOUT,
                    ).await,
                    Err(error) => Err(error),
                };
                let projection_shutdown = match (result.is_ok(), removed_manifest) {
                    (true, Some(manifest)) => {
                        AgentProjectionShutdown::ManifestRevoked(manifest)
                    }
                    _ => AgentProjectionShutdown::Invalidate,
                };
                break (result, projection_shutdown);
            }
        }
    };
    drop(listener);
    #[cfg(test)]
    state
        .control_test_hooks
        .runtime_before_cleanup
        .pause_if_armed()
        .await;
    settle_worker_runtime_with_projection(&state, supervised_tasks, result, projection_shutdown)
        .await
}

/// A child that exits during custody activation finalizes before serving is
/// announced, so its natural-exit schedule is deferred. Recover that case now
/// that the serving loop can observe the shutdown notification.
pub(crate) fn recover_deferred_natural_exit_shutdown(state: &Arc<WorkerState>) {
    if lock_recover(&state.exit).is_some() {
        state.schedule_natural_exit_shutdown();
    }
}

#[cfg(test)]
pub(crate) async fn settle_worker_runtime<T: 'static>(
    state: &WorkerState,
    supervised_tasks: JoinSet<T>,
    result: Result<()>,
) -> Result<()> {
    settle_worker_runtime_with_projection(
        state,
        supervised_tasks,
        result,
        AgentProjectionShutdown::Invalidate,
    )
    .await
}

pub(crate) async fn settle_worker_runtime_with_projection<T: 'static>(
    state: &WorkerState,
    mut supervised_tasks: JoinSet<T>,
    result: Result<()>,
    projection_shutdown: AgentProjectionShutdown,
) -> Result<()> {
    state.commit_response_shutdown();
    let codex_ingress = state.shutdown_codex_hook_ingress().await;
    let projection_drain = state.codex_hook_ingress.wait_projection().await;
    // A bounded wait preserves admitted response writes. On timeout, cancelling
    // connection tasks releases their tickets before the runtime result escapes.
    let _ = state
        .wait_for_response_drain(crate::session_timing::WORKER_RESPONSE_DRAIN_TIMEOUT)
        .await;
    abort_and_drain_tasks(&mut supervised_tasks).await;
    let projection =
        codex_hook::invalidate_projection_for_shutdown(state, projection_shutdown).await;
    let result = preserve_cleanup_failure(result, codex_ingress, CODEX_INGRESS_SHUTDOWN_FAILURE);
    let result =
        preserve_cleanup_failure(result, projection_drain, CODEX_PROJECTION_SHUTDOWN_FAILURE);
    preserve_cleanup_failure(
        result,
        projection.map(|_| ()),
        CODEX_PROJECTION_SHUTDOWN_FAILURE,
    )
}

pub(crate) const CODEX_INGRESS_SHUTDOWN_FAILURE: &str = "Codex hook ingress shutdown failed";
pub(crate) const CODEX_PROJECTION_SHUTDOWN_FAILURE: &str = "Codex projection shutdown failed";
pub(crate) const ORPHAN_EXIT_DURABILITY_TIMEOUT: &str =
    "orphaned worker exit did not become durable before its deadline";
pub(crate) const ORPHAN_EXIT_EVENT_STREAM_CLOSED: &str =
    "worker exit event stream closed before orphan finalization";

pub(crate) fn preserve_cleanup_failure(
    primary: Result<()>,
    cleanup: Result<()>,
    detail: &'static str,
) -> Result<()> {
    match (primary, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(cleanup)) => Err(cleanup).context(detail),
        (Err(primary), Ok(())) => Err(primary),
        (Err(primary), Err(cleanup)) => Err(primary.context(format!("{detail}: {cleanup:#}"))),
    }
}

pub(crate) async fn wait_for_orphan_exit_durability(
    state: &WorkerState,
    events: &mut broadcast::Receiver<WorkerEvent>,
    timeout: Duration,
) -> Result<()> {
    tokio::time::timeout(timeout, async {
        loop {
            if lock_recover(&state.exit).is_some() {
                return Ok(());
            }
            match events.recv().await {
                Ok(WorkerEvent::Exited(_)) => return Ok(()),
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => {
                    bail!(ORPHAN_EXIT_EVENT_STREAM_CLOSED)
                }
            }
        }
    })
    .await
    .context(ORPHAN_EXIT_DURABILITY_TIMEOUT)?
}
