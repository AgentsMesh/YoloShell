pub(crate) struct HostIngressTasks {
    pub(crate) clients: tokio::task::JoinSet<()>,
    pub(crate) idle_shutdown: tokio::task::JoinSet<()>,
    #[cfg(test)]
    pub(crate) fault:
        Option<std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>>,
}

impl HostIngressTasks {
    pub(crate) fn new() -> Self {
        Self {
            clients: tokio::task::JoinSet::new(),
            idle_shutdown: tokio::task::JoinSet::new(),
            #[cfg(test)]
            fault: None,
        }
    }
}

pub(crate) async fn run_host_ingress(
    host: Arc<Host>,
    listener: tokio::net::UnixListener,
    armed: tokio::sync::oneshot::Sender<IngressArmed>,
    shutdown: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    run_host_ingress_runtime(host, listener, armed, shutdown, HostIngressTasks::new()).await
}

pub(crate) async fn run_host_ingress_runtime(
    host: Arc<Host>,
    listener: tokio::net::UnixListener,
    armed: tokio::sync::oneshot::Sender<IngressArmed>,
    shutdown: tokio::sync::oneshot::Receiver<()>,
    mut tasks: HostIngressTasks,
) -> Result<()> {
    let caught = terminal_containment::catch_future_detail(run_host_ingress_loop(
        &host, listener, armed, shutdown, &mut tasks,
    ))
    .await;
    let outcome = match caught {
        Ok(outcome) => outcome,
        Err(detail) => Err(anyhow::Error::msg(detail).context(HOST_INGRESS_LOOP_PANIC_CONTEXT)),
    };
    shutdown_ingress_tasks(&host, &mut tasks.clients, &mut tasks.idle_shutdown).await;
    outcome
}

pub(crate) async fn run_host_ingress_loop(
    host: &Arc<Host>,
    listener: tokio::net::UnixListener,
    armed: tokio::sync::oneshot::Sender<IngressArmed>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
    tasks: &mut HostIngressTasks,
) -> Result<()> {
    let clients = Arc::new(tokio::sync::Semaphore::new(MAX_HOST_CLIENTS));
    let mut idle_shutdown_armed = false;
    let mut backoff =
        crate::ingress::AcceptBackoff::new(HOST_ACCEPT_INITIAL_BACKOFF, HOST_ACCEPT_MAX_BACKOFF);
    let mut first = poll_accept_once(&listener, armed).await;
    let mut namespace_check = tokio::time::interval(WATCHDOG_INTERVAL);
    namespace_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    #[cfg(test)]
    if let Some(fault) = tasks.fault.take() {
        fault.await?;
    }
    loop {
        if let Some(accepted) = first.take() {
            handle_host_accept(host, &clients, &mut tasks.clients, &mut backoff, accepted).await;
            continue;
        }
        tokio::select! {
            biased;
            _ = &mut shutdown => return Ok(()),
            _ = namespace_check.tick() => {
                host.verify_namespace()?;
                if idle_shutdown_armed {
                    schedule_idle_shutdown(
                        host,
                        &clients,
                        &tasks.clients,
                        &mut tasks.idle_shutdown,
                    );
                }
            },
            joined = tasks.clients.join_next(), if !tasks.clients.is_empty() => {
                report_joined_task_panic(joined, HOST_CLIENT_JOIN_PANIC_LOG);
                idle_shutdown_armed = tasks.clients.is_empty();
                schedule_idle_shutdown(
                    host,
                    &clients,
                    &tasks.clients,
                    &mut tasks.idle_shutdown,
                );
            }
            joined = tasks.idle_shutdown.join_next(), if !tasks.idle_shutdown.is_empty() => {
                report_joined_task_panic(joined, HOST_IDLE_SHUTDOWN_JOIN_PANIC_LOG);
            }
            accepted = listener.accept() => {
                tasks.idle_shutdown.abort_all();
                while let Some(joined) = tasks.idle_shutdown.join_next().await {
                    report_joined_task_panic(Some(joined), HOST_IDLE_SHUTDOWN_JOIN_PANIC_LOG);
                }
                handle_host_accept(
                    host,
                    &clients,
                    &mut tasks.clients,
                    &mut backoff,
                    accepted.map(|(stream, address)| (UnixStream::from(stream), address)),
                ).await;
            }
        }
    }
}

pub(crate) fn report_joined_task_panic(
    joined: Option<std::result::Result<(), tokio::task::JoinError>>,
    message: &'static str,
) {
    if matches!(joined, Some(Err(error)) if error.is_panic()) {
        tracing::error!(message = message);
    }
}
