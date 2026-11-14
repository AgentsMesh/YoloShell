pub(crate) struct HostIngress {
    pub(crate) task: tokio::task::JoinHandle<Result<()>>,
    pub(crate) shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

pub(crate) struct IngressArmed;

impl HostIngress {
    pub(crate) fn spawn(
        host: Arc<Host>,
        listener: tokio::net::UnixListener,
        armed: tokio::sync::oneshot::Sender<IngressArmed>,
    ) -> Self {
        let (shutdown, receiver) = tokio::sync::oneshot::channel();
        Self {
            task: tokio::spawn(run_host_ingress(host, listener, armed, receiver)),
            shutdown: Some(shutdown),
        }
    }

    pub(crate) async fn shutdown(&mut self) -> Result<()> {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        match (&mut self.task).await {
            Err(error) => Err(error).context(HOST_INGRESS_SHUTDOWN_CONTEXT),
            Ok(outcome) => outcome.context(HOST_INGRESS_SHUTDOWN_CONTEXT),
        }
    }

    pub(crate) fn disarm(&mut self) {
        self.shutdown.take();
    }
}

impl Drop for HostIngress {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

include!("server_startup.rs");

impl Host {
    pub async fn serve(self) -> Result<()> {
        self.serve_with_stop(std::future::pending()).await
    }

    pub async fn serve_with_stop<F: std::future::Future<Output = ()>>(self, stop: F) -> Result<()> {
        let mut shutdown_signals = HostShutdownSignals::install()?;
        let shutdown = async { tokio::select! { _ = shutdown_signals.wait() => {}, _ = stop => {} } };
        self.serve_until(shutdown, ServingHost::start_maintenance)
            .await
    }

    #[cfg(test)]
    pub(crate) async fn serve_until_with_maintenance<F>(
        self,
        shutdown: F,
        plan: crate::maintenance::MaintenancePlan,
    ) -> Result<()>
    where
        F: std::future::Future<Output = ()>,
    {
        self.serve_until(shutdown, move |serving| {
            serving.start_maintenance_plan(plan);
            Ok(())
        })
        .await
    }

    pub(crate) async fn serve_until<F, S>(self, shutdown: F, start_maintenance: S) -> Result<()>
    where
        F: std::future::Future<Output = ()>,
        S: FnOnce(&mut ServingHost) -> Result<()>,
    {
        #[cfg(test)]
        return self
            .serve_until_inner(shutdown, start_maintenance, None)
            .await;
        #[cfg(not(test))]
        return self.serve_until_inner(shutdown, start_maintenance).await;
    }

    #[cfg(test)]
    pub(crate) async fn serve_until_observed<F>(
        self,
        shutdown: F,
        started: tokio::sync::oneshot::Sender<()>,
    ) -> Result<()>
    where
        F: std::future::Future<Output = ()>,
    {
        self.serve_until_inner(shutdown, |_| Ok(()), Some(started))
            .await
    }

    pub(crate) async fn serve_until_inner<F, S>(
        self,
        shutdown: F,
        start_maintenance: S,
        #[cfg(test)] started: Option<tokio::sync::oneshot::Sender<()>>,
    ) -> Result<()>
    where
        F: std::future::Future<Output = ()>,
        S: FnOnce(&mut ServingHost) -> Result<()>,
    {
        self.verify_namespace()?;
        let parent = self.socket.parent().context("host socket has no parent")?;
        let directory = self.namespace.directory(parent)?;
        let (listener, publication) = crate::paths::socket_activation::publish(
            directory,
            &self.socket,
            SOCKET_PROBE_TIMEOUT,
            HOST_SOCKET_LIVE_CONTEXT,
        )
        .await?;
        let mut serving = ServingHost::capture(self, publication);
        serving.host().verify_namespace()?;
        let host = Arc::clone(serving.host());
        let (armed, ready) = tokio::sync::oneshot::channel();
        let mut ingress = HostIngress::spawn(Arc::clone(&host), listener, armed);
        if let Err(error) = ready.await.context(HOST_INGRESS_ARM_CONTEXT) {
            return stop_ingress_after_startup_failure(&mut ingress, Err(error), Ok(())).await;
        }
        if let Err(error) = start_maintenance(&mut serving) {
            return stop_after_maintenance_start_failure(&mut ingress, host, &mut serving, error)
                .await;
        }
        let watchdog = host.spawn_watchdog();
        #[cfg(test)]
        if let Some(started) = started {
            let _ = started.send(());
        }
        tracing::info!(
            socket = %host.socket.display(),
            generation = host.controller_generation,
            "Yolo host accepting clients; recovery supervisor started"
        );
        let drain = host.request_drain.clone();
        let service_shutdown = async move {
            tokio::select! {
                biased;
                outcome = drain.wait_for_shutdown() => outcome.map_err(anyhow::Error::msg),
                _ = shutdown => Ok(()),
            }
        };
        tokio::pin!(service_shutdown);
        let (outcome, ingress_finished) = tokio::select! {
            biased;
            outcome = &mut service_shutdown => (outcome, false),
            outcome = &mut ingress.task => {
                let result = (|| -> Result<()> {
                    outcome.context(HOST_INGRESS_JOIN_CONTEXT)??;
                    bail!(HOST_INGRESS_STOPPED_CONTEXT);
                })();
                (result, true)
            }
        };
        let mut maintenance = serving.take_maintenance();
        if let Some(worker) = maintenance.as_ref() {
            worker.request_shutdown();
        }
        let ingress_outcome = if ingress_finished {
            ingress.disarm();
            Ok(())
        } else {
            ingress.shutdown().await
        };
        watchdog.cancel_and_wait().await;
        // A scanner may stall inside a chunk, so release all Host ownership before joining it.
        drop(ingress);
        drop(host);
        serving.release();
        drop(serving);
        let maintenance_outcome = match maintenance.as_mut() {
            Some(worker) => worker.shutdown().await,
            None => Ok(()),
        };
        host_shutdown_result(outcome, ingress_outcome, maintenance_outcome)?;
        tracing::info!("Yolo host shutting down; session workers remain attached to their PTYs");
        Ok(())
    }
}

