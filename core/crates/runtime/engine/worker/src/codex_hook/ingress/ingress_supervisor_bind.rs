type IngressStartResult = std::result::Result<Arc<super::router::CodexSessionRouter>, Arc<str>>;

impl CodexHookIngressSupervisor {
    pub(in crate::worker) async fn start_native(
        &self,
        namespace: &crate::worker::WorkerNamespace,
        paths: &crate::paths::Paths,
        terminal_id: uuid::Uuid,
        runtime_epoch: uuid::Uuid,
        shell: crate::process::ProcessIdentity,
        hook_executable: &std::path::Path,
        agent_registry: std::sync::Arc<crate::agent::AgentRegistry>,
    ) -> anyhow::Result<()> {
        let request = super::server::CodexHookBindRequest::capture_native(
            namespace,
            paths,
            Arc::clone(&self.registry),
            terminal_id,
            runtime_epoch,
            shell,
            hook_executable,
            Arc::new(crate::agent::RuntimeProcessSource::new(
                agent_registry.process_environment_keys(),
            )),
        )?;
        self.start_request(request).await.map(|_| ())
    }

    #[cfg(test)]
    pub(in crate::worker::codex_hook) async fn bind_test(
        &self,
        namespace: &crate::worker::WorkerNamespace,
        paths: &crate::paths::Paths,
        terminal_id: uuid::Uuid,
        runtime_epoch: uuid::Uuid,
        shell: crate::process::ProcessIdentity,
        hook_executable: &std::path::Path,
        process_source: Arc<dyn crate::agent::ProcessSource>,
    ) -> anyhow::Result<super::server::CodexHookServer> {
        super::server::CodexHookBindRequest::capture_native(
            namespace,
            paths,
            Arc::clone(&self.registry),
            terminal_id,
            runtime_epoch,
            shell,
            hook_executable,
            process_source,
        )?
        .bind(super::cancellation::IngressCancellation::default())
        .await
    }

    #[cfg(test)]
    pub(in crate::worker::codex_hook) async fn start_test(
        &self,
        namespace: &crate::worker::WorkerNamespace,
        paths: &crate::paths::Paths,
        terminal_id: uuid::Uuid,
        runtime_epoch: uuid::Uuid,
        shell: crate::process::ProcessIdentity,
        hook_executable: &std::path::Path,
        process_source: Arc<dyn crate::agent::ProcessSource>,
    ) -> anyhow::Result<Arc<super::router::CodexSessionRouter>> {
        let request = super::server::CodexHookBindRequest::capture_native(
            namespace,
            paths,
            Arc::clone(&self.registry),
            terminal_id,
            runtime_epoch,
            shell,
            hook_executable,
            process_source,
        )?;
        self.start_request(request).await
    }

    async fn start_request(
        &self,
        request: super::server::CodexHookBindRequest,
    ) -> anyhow::Result<Arc<super::router::CodexSessionRouter>> {
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let operation_inner = Arc::downgrade(&self.inner);
        let monitor_inner = Arc::downgrade(&self.inner);
        let (operation, launch) =
            IngressOperation::spawn(monitor_inner, move |cancellation, identity| async move {
                supervise_ingress(request, operation_inner, cancellation, identity, started_tx)
                    .await
            });
        if self.inner.reserve(Arc::clone(&operation)).is_err() {
            drop(launch);
            anyhow::bail!(super::constants::CODEX_INGRESS_ALREADY_RUNNING_ERROR);
        }
        let _ = launch.send(());
        match started_rx.await {
            Ok(Ok(router)) => Ok(router),
            Ok(Err(detail)) => {
                let _ = operation.wait().await;
                self.inner.clear(&operation);
                anyhow::bail!(detail.to_string())
            }
            Err(error) => {
                let _ = operation.wait().await;
                self.inner.clear(&operation);
                Err(anyhow::Error::new(error)
                    .context(super::constants::CODEX_INGRESS_START_RESULT_ERROR))
            }
        }
    }
}

impl crate::worker::WorkerState {
    pub(in crate::worker) async fn shutdown_codex_hook_ingress(&self) -> anyhow::Result<()> {
        self.codex_hook_ingress.shutdown().await
    }
}
