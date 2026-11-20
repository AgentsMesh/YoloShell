include!("ingress_runtime.rs");

pub(crate) fn schedule_idle_shutdown(
    host: &Arc<Host>,
    clients: &Arc<tokio::sync::Semaphore>,
    client_tasks: &tokio::task::JoinSet<()>,
    shutdown_tasks: &mut tokio::task::JoinSet<()>,
) {
    if !client_tasks.is_empty() || !shutdown_tasks.is_empty() {
        return;
    }
    let host = Arc::clone(host);
    let clients = Arc::clone(clients);
    shutdown_tasks.spawn(async move {
        tokio::time::sleep(HOST_IDLE_SHUTDOWN_QUIET_PERIOD).await;
        let preparation = tokio::time::timeout(
            HOST_IDLE_SHUTDOWN_CHECK_TIMEOUT,
            host.prepare_quiescent_shutdown(clients),
        )
        .await;
        if let Ok(Ok(preparation)) = preparation {
            preparation.commit()
        }
    });
}

#[cfg(test)]
pub(crate) async fn drain_host_clients(host: &Host, tasks: &mut tokio::task::JoinSet<()>) {
    host.request_drain.commit_response_shutdown();
    drain_committed_host_clients(host, tasks).await;
}

pub(crate) async fn drain_committed_host_clients(
    host: &Host,
    tasks: &mut tokio::task::JoinSet<()>,
) {
    await_host_response_drain(&host.request_drain, HOST_RESPONSE_DRAIN_TIMEOUT).await;
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
}

pub(crate) async fn shutdown_ingress_tasks(
    host: &Host,
    client_tasks: &mut tokio::task::JoinSet<()>,
    idle_shutdown_tasks: &mut tokio::task::JoinSet<()>,
) {
    host.request_drain.commit_response_shutdown();
    idle_shutdown_tasks.shutdown().await;
    drain_committed_host_clients(host, client_tasks).await;
}

pub(crate) async fn await_host_response_drain(
    drain: &request_drain::HostRequestDrain,
    timeout: std::time::Duration,
) -> bool {
    let drained = drain.wait_for_response_drain(timeout).await;
    if !drained {
        terminal_containment::trace(|| {
            tracing::warn!("{HOST_RESPONSE_DRAIN_TIMEOUT_LOG}");
        });
    }
    drained
}

pub(crate) async fn poll_accept_once(
    listener: &tokio::net::UnixListener,
    armed: tokio::sync::oneshot::Sender<IngressArmed>,
) -> Option<std::io::Result<(UnixStream, tokio::net::unix::SocketAddr)>> {
    let mut armed = Some(armed);
    std::future::poll_fn(|context| {
        let accepted = listener
            .poll_accept(context)
            .map(|result| result.map(|(stream, address)| (UnixStream::from(stream), address)));
        if let Some(sender) = armed.take() {
            let _ = sender.send(IngressArmed);
        }
        std::task::Poll::Ready(match accepted {
            std::task::Poll::Ready(result) => Some(result),
            std::task::Poll::Pending => None,
        })
    })
    .await
}

pub(crate) async fn handle_host_accept(
    host: &Arc<Host>,
    clients: &Arc<tokio::sync::Semaphore>,
    client_tasks: &mut tokio::task::JoinSet<()>,
    backoff: &mut crate::ingress::AcceptBackoff,
    accepted: std::io::Result<(UnixStream, tokio::net::unix::SocketAddr)>,
) {
    match accepted {
        Ok((stream, _)) => {
            backoff.reset();
            let Some(permit) = crate::ingress::try_admit(clients) else {
                tracing::warn!("host client limit reached");
                return;
            };
            let host = Arc::clone(host);
            client_tasks.spawn(async move {
                let _permit = permit;
                match contain_panic(host.handle_client(stream)).await {
                    Ok(Err(error))
                        if !matches!(
                            error.downcast_ref::<crate::protocol::ProtocolError>(),
                            Some(crate::protocol::ProtocolError::EndOfStream)
                        ) =>
                    {
                        tracing::debug!(%error, "host client disconnected")
                    }
                    Err(_) => tracing::error!("host client task panicked"),
                    _ => {}
                }
            });
        }
        Err(error) => {
            let delay = backoff.next_delay();
            tracing::warn!(%error, ?delay, "host accept failed; retrying");
            tokio::time::sleep(delay).await;
        }
    }
}
