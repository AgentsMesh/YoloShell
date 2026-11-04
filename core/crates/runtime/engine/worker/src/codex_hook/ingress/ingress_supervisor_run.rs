type IngressBoundServer = (
    super::server::CodexHookServer,
    tokio::sync::OwnedMutexGuard<()>,
);

async fn supervise_ingress(
    request: super::server::CodexHookBindRequest,
    inner: std::sync::Weak<IngressSupervisorInner>,
    cancellation: super::cancellation::IngressCancellation,
    operation_identity: Arc<()>,
    started: tokio::sync::oneshot::Sender<IngressStartResult>,
) -> anyhow::Result<()> {
    let Some(supervisor) = inner.upgrade() else {
        let _ = started.send(Err(start_cancelled()));
        return Ok(());
    };
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => {
            let _ = started.send(Err(start_cancelled()));
            return Ok(());
        }
        _ = supervisor.wait_start_gate() => {}
    }
    drop(supervisor);
    supervise_rebinds(request, inner, cancellation, operation_identity, started).await
}

async fn supervise_rebinds(
    request: super::server::CodexHookBindRequest,
    inner: std::sync::Weak<IngressSupervisorInner>,
    cancellation: super::cancellation::IngressCancellation,
    operation_identity: Arc<()>,
    started: tokio::sync::oneshot::Sender<IngressStartResult>,
) -> anyhow::Result<()> {
    let mut started = Some(started);
    let mut initial_bind = true;
    let mut failures = u32::default();
    loop {
        let (server, publication_guard) =
            match bind_once(&request, &inner, &cancellation, &operation_identity).await {
                Ok(Some(bound)) => bound,
                Ok(None) => {
                    send_start_cancelled(&mut started);
                    return Ok(());
                }
                Err(error) if initial_bind => {
                    send_start_failure(&mut started, &error);
                    return Err(error);
                }
                Err(error) => {
                    if !wait_to_restart(&inner, &cancellation, &mut failures, &error).await {
                        return Ok(());
                    }
                    continue;
                }
            };
        failures = u32::default();
        let Some(supervisor) = inner.upgrade() else {
            send_start_cancelled(&mut started);
            return Ok(());
        };
        let router = server.router();
        let listener_identity = Arc::new(());
        #[cfg(test)]
        let (listener, completion) = spawn_listener(
            server,
            cancellation.clone(),
            Arc::clone(&supervisor.test_control),
        );
        #[cfg(not(test))]
        let (listener, completion) = spawn_listener(server, cancellation.clone());
        let installed = supervisor.install_listener(
            &operation_identity,
            Arc::clone(&listener_identity),
            listener.abort_handle(),
            completion,
            Arc::clone(&router),
        );
        drop(publication_guard);
        drop(supervisor);
        if !installed {
            listener.abort();
            let _ = listener.await;
            send_start_cancelled(&mut started);
            return Ok(());
        }
        if let Some(started) = started.take() {
            let _ = started.send(Ok(router));
        }
        initial_bind = false;
        let completed = listener.await;
        let should_restart = inner.upgrade().is_some_and(|supervisor| {
            supervisor.finish_listener(&operation_identity, &listener_identity)
        });
        if cancellation.is_cancelled() {
            return listener_settlement(completed);
        }
        if !should_restart {
            return Ok(());
        }
        let error = listener_failure(completed);
        if !wait_to_restart(&inner, &cancellation, &mut failures, &error).await {
            return Ok(());
        }
    }
}

async fn bind_once(
    request: &super::server::CodexHookBindRequest,
    _inner: &std::sync::Weak<IngressSupervisorInner>,
    cancellation: &super::cancellation::IngressCancellation,
    operation_identity: &Arc<()>,
) -> anyhow::Result<Option<IngressBoundServer>> {
    if cancellation.is_cancelled() {
        return Ok(None);
    }
    let Some(publication_gate) = _inner
        .upgrade()
        .map(|supervisor| Arc::clone(&supervisor.publication_gate))
    else {
        return Ok(None);
    };
    let publication_guard = tokio::select! {
        biased;
        _ = cancellation.cancelled() => return Ok(None),
        guard = publication_gate.lock_owned() => guard,
    };
    if !_inner
        .upgrade()
        .is_some_and(|supervisor| supervisor.permits_bind(operation_identity))
    {
        return Ok(None);
    }
    #[cfg(test)]
    if _inner
        .upgrade()
        .is_some_and(|supervisor| supervisor.test_control.fail_bind())
    {
        anyhow::bail!(INGRESS_TEST_BIND_FAILURE);
    }
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => Ok(None),
        result = request.bind(cancellation.clone()) => {
            result.map(|server| Some((server, publication_guard)))
        },
    }
}

#[cfg(not(test))]
fn spawn_listener(
    server: super::server::CodexHookServer,
    cancellation: super::cancellation::IngressCancellation,
) -> (
    tokio::task::JoinHandle<anyhow::Result<()>>,
    super::ingress_listener_completion::IngressListenerCompletion,
) {
    let (completion, guard) =
        super::ingress_listener_completion::IngressListenerCompletion::pending();
    let listener = tokio::spawn(async move {
        let _guard = guard;
        super::ingress_task::run(server, cancellation).await
    });
    (listener, completion)
}

#[cfg(test)]
fn spawn_listener(
    server: super::server::CodexHookServer,
    cancellation: super::cancellation::IngressCancellation,
    control: Arc<IngressTestControl>,
) -> (
    tokio::task::JoinHandle<anyhow::Result<()>>,
    super::ingress_listener_completion::IngressListenerCompletion,
) {
    let (completion, guard) =
        super::ingress_listener_completion::IngressListenerCompletion::pending();
    let listener = tokio::spawn(async move {
        let _guard = guard;
        tokio::select! {
            biased;
            fault = control.next_listener_fault() => fault.into_result(),
            result = super::ingress_task::run(server, cancellation) => result,
        }
    });
    (listener, completion)
}
