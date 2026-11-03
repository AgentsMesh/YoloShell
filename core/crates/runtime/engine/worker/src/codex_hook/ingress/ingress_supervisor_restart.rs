fn listener_settlement(
    completed: std::result::Result<anyhow::Result<()>, tokio::task::JoinError>,
) -> anyhow::Result<()> {
    completed
        .map_err(anyhow::Error::new)?
        .context(super::constants::CODEX_INGRESS_TASK_ERROR)
}

fn listener_failure(
    completed: std::result::Result<anyhow::Result<()>, tokio::task::JoinError>,
) -> anyhow::Error {
    match completed {
        Ok(Ok(())) => anyhow::anyhow!(super::constants::CODEX_INGRESS_TASK_ERROR),
        Ok(Err(error)) => error,
        Err(error) if error.is_cancelled() => {
            anyhow::anyhow!(super::constants::CODEX_INGRESS_LISTENER_ABORTED_ERROR)
        }
        Err(error) => anyhow::Error::new(error).context(super::constants::CODEX_INGRESS_TASK_ERROR),
    }
}

async fn wait_to_restart(
    _inner: &std::sync::Weak<IngressSupervisorInner>,
    cancellation: &super::cancellation::IngressCancellation,
    failures: &mut u32,
    error: &anyhow::Error,
) -> bool {
    *failures = failures.saturating_add(super::constants::CODEX_INGRESS_RESTART_FAILURE_INCREMENT);
    let failure = crate::worker::supervision::TaskFailure::message(
        super::constants::CODEX_INGRESS_TASK_NAME,
        crate::worker::supervision::error_ref_detail(error),
    );
    let policy = crate::worker::supervision::RestartPolicy {
        initial_delay: super::constants::CODEX_INGRESS_RESTART_INITIAL_DELAY,
        max_delay: super::constants::CODEX_INGRESS_RESTART_MAX_DELAY,
    };
    let delay = crate::worker::supervision::report_restart(&failure, policy, *failures);
    #[cfg(test)]
    if let Some(supervisor) = _inner.upgrade() {
        supervisor.test_control.record_restart_delay(delay);
    }
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => false,
        _ = tokio::time::sleep(delay) => true,
    }
}

fn send_start_failure(
    started: &mut Option<tokio::sync::oneshot::Sender<IngressStartResult>>,
    error: &anyhow::Error,
) {
    if let Some(started) = started.take() {
        let _ = started.send(Err(Arc::from(format!("{error:#}"))));
    }
}

fn send_start_cancelled(started: &mut Option<tokio::sync::oneshot::Sender<IngressStartResult>>) {
    if let Some(started) = started.take() {
        let _ = started.send(Err(start_cancelled()));
    }
}

fn start_cancelled() -> Arc<str> {
    Arc::from(super::constants::CODEX_INGRESS_START_CANCELLED_ERROR)
}
