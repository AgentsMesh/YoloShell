pub(crate) fn handle_accept_result(
    accepted: io::Result<(UnixStream, tokio::net::unix::SocketAddr)>,
    tasks: &mut JoinSet<Option<supervision::TaskFailure>>,
    state: &Arc<WorkerState>,
    clients: &Arc<Semaphore>,
    failure_count: &mut u32,
) -> Option<Duration> {
    match accepted {
        Ok((stream, _)) => {
            *failure_count = 0;
            let Some(permit) = crate::ingress::try_admit(clients) else {
                tracing::warn!("worker client limit reached");
                drop(stream);
                return None;
            };
            let state = Arc::clone(state);
            supervision::spawn_task(tasks, supervision::CONNECTION_TASK, async move {
                let _permit = permit;
                match handle_connection(stream, state).await {
                    Err(error)
                        if matches!(
                            error.downcast_ref::<ProtocolError>(),
                            Some(ProtocolError::EndOfStream)
                        ) =>
                    {
                        Ok(())
                    }
                    result => result,
                }
            });
            None
        }
        Err(error) => {
            *failure_count = failure_count.saturating_add(1);
            let failure =
                supervision::TaskFailure::message(supervision::ACCEPT_TASK, error.to_string());
            let delay = supervision::report_restart(
                &failure,
                supervision::RestartPolicy::default(),
                *failure_count,
            );
            Some(delay)
        }
    }
}

pub(crate) async fn wait_for_accept_retry(
    retry: &mut Option<std::pin::Pin<Box<tokio::time::Sleep>>>,
) {
    match retry {
        Some(delay) => delay.await,
        None => std::future::pending().await,
    }
}

pub(crate) async fn abort_and_drain_tasks<T: 'static>(tasks: &mut JoinSet<T>) {
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
}

pub(crate) fn handle_task_completion(
    completed: Option<
        std::result::Result<Option<supervision::TaskFailure>, tokio::task::JoinError>,
    >,
    tasks: &mut JoinSet<Option<supervision::TaskFailure>>,
    state: &Arc<WorkerState>,
    monitor: Option<&AgentMonitorConfig>,
    shell_pid: u32,
) {
    let failure = match completed {
        Some(Ok(Some(failure))) => Some(failure),
        Some(Err(error)) => Some(supervision::TaskFailure::message(
            supervision::TASK_SUPERVISOR,
            error.to_string(),
        )),
        _ => None,
    };
    let Some(failure) = failure else {
        return;
    };
    let restart_monitor = failure.task == supervision::AGENT_MONITOR_TASK
        && !state.child_waited.load(Ordering::Acquire);
    supervision::report_contained(&failure);
    if restart_monitor && let Some(config) = monitor.cloned() {
        spawn_agent_monitor_task(tasks, Arc::clone(state), config, shell_pid);
    }
}
