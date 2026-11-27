impl RemoteBroker {
    async fn run_server<F>(
        mut self,
        shutdown: F,
        adoption: Option<tokio::sync::oneshot::Receiver<super::BrokerAdoptionAcknowledgement>>,
        runtime_limit: usize,
        attachment_limit: usize,
        management_limit: usize,
        mut maintenance: BrokerMaintenance,
    ) -> Result<()>
    where
        F: Future<Output = ()> + Send,
    {
        let runtime_tasks = Arc::new(BrokerTasks::default());
        let runtime_proxies = Arc::new(BrokerTasks::default());
        let management_tasks = Arc::new(BrokerTasks::default());
        let hub_tasks = Arc::new(HubTasks::new());
        let admission = Arc::new(BrokerAdmission::new(adoption.is_some()));
        let instance_id = self.pool.instance_id();
        let adoption_task = adoption.map(|receiver| {
            let admission = Arc::clone(&admission);
            tokio::spawn(async move {
                if let Ok(acknowledge) = receiver.await {
                    let _ = tokio::task::spawn_blocking(move || {
                        admission.adopt(instance_id, acknowledge);
                    })
                    .await;
                }
            })
        });
        tokio::pin!(shutdown);
        let startup_lock = self.startup_lock.take();
        let sockets = BrokerSockets::bind_authorized(
            &self.paths,
            self.launch_authority,
            startup_lock,
            self.startup_deadline,
        )
        .await?;
        let cause = {
            let namespace = sockets.namespace();
            let runtime_clients = Arc::new(Semaphore::new(runtime_limit));
            let runtime_attachments = Arc::new(Semaphore::new(attachment_limit));
            let management_clients = Arc::new(Semaphore::new(management_limit));
            let internal_shutdown = self.shutdown.notified();
            let internal_retirement = self.retirement.notified();
            let runtime_listener = serve_listener(
                &sockets.runtime,
                Arc::clone(&namespace),
                RUNTIME_LISTENER_LABEL,
                |stream: LocalUnixStream| {
                    self.spawn_runtime(
                        stream,
                        Arc::clone(&runtime_clients),
                        Arc::clone(&runtime_attachments),
                        Arc::clone(&admission),
                        &runtime_tasks,
                        Arc::clone(&runtime_proxies),
                    )
                },
            );
            let management_listener = serve_listener(
                &sockets.management,
                Arc::clone(&namespace),
                MANAGEMENT_LISTENER_LABEL,
                |stream: LocalUnixStream| {
                    self.spawn_management(
                        stream,
                        Arc::clone(&management_clients),
                        Arc::clone(&admission),
                        &management_tasks,
                        Arc::clone(&hub_tasks),
                    )
                },
            );
            let namespace_watcher = namespace.watch();
            tokio::pin!(
                internal_shutdown,
                internal_retirement,
                runtime_listener,
                management_listener,
                namespace_watcher
            );
            let cause = tokio::select! {
                result = &mut runtime_listener => namespace_shutdown(result),
                result = &mut management_listener => namespace_shutdown(result),
                result = &mut namespace_watcher => namespace_shutdown(result),
                _ = shutdown.as_mut() => BrokerShutdownCause::External,
                _ = &mut internal_shutdown => BrokerShutdownCause::Rotation,
                _ = &mut internal_retirement => BrokerShutdownCause::Retirement,
            };
            admission.close();
            cause
        };
        if let Some(task) = adoption_task {
            task.abort();
        }
        maintenance.request_shutdown();
        let credential_retirement = matches!(&cause, BrokerShutdownCause::Retirement);
        let retirement_lock = if credential_retirement {
            Some(sockets.close_for_retirement())
        } else {
            drop(sockets);
            None
        };
        let mut retirement_settled = true;
        match &cause {
            BrokerShutdownCause::Rotation => {
                tokio::join!(
                    management_tasks
                        .finish_bounded(MANAGEMENT_DRAIN_TIMEOUT, MANAGEMENT_LISTENER_LABEL),
                    runtime_tasks
                        .finish_bounded(RUNTIME_SETUP_DRAIN_TIMEOUT, RUNTIME_LISTENER_LABEL),
                );
                runtime_proxies
                    .finish_with_hub_or_external(&hub_tasks.proxies, shutdown.as_mut())
                    .await;
            }
            BrokerShutdownCause::Retirement => {
                let runtime_deadline = tokio::time::Instant::now() + EXTERNAL_RUNTIME_DRAIN_TIMEOUT;
                let (management, runtime) = tokio::join!(
                    management_tasks
                        .finish_bounded(MANAGEMENT_DRAIN_TIMEOUT, MANAGEMENT_LISTENER_LABEL),
                    async {
                        let setup = runtime_tasks
                            .finish_before(runtime_deadline, RUNTIME_LISTENER_LABEL)
                            .await;
                        let proxy = runtime_proxies
                            .finish_before(runtime_deadline, RUNTIME_LISTENER_LABEL)
                            .await;
                        let hub = hub_tasks
                            .proxies
                            .finish_before(
                                runtime_deadline,
                                super::hub_peer_constants::HUB_TUNNEL_LABEL,
                            )
                            .await;
                        setup && proxy && hub
                    },
                );
                retirement_settled = management && runtime;
            }
            BrokerShutdownCause::External | BrokerShutdownCause::NamespaceLost(_) => {
                let runtime_deadline = tokio::time::Instant::now() + EXTERNAL_RUNTIME_DRAIN_TIMEOUT;
                tokio::join!(
                    management_tasks
                        .finish_bounded(MANAGEMENT_DRAIN_TIMEOUT, MANAGEMENT_LISTENER_LABEL),
                    async {
                        runtime_tasks
                            .finish_before(runtime_deadline, RUNTIME_LISTENER_LABEL)
                            .await;
                        runtime_proxies
                            .finish_before(runtime_deadline, RUNTIME_LISTENER_LABEL)
                            .await;
                        hub_tasks
                            .proxies
                            .finish_before(
                                runtime_deadline,
                                super::hub_peer_constants::HUB_TUNNEL_LABEL,
                            )
                            .await;
                    },
                );
            }
        }
        let (maintenance, cleanup) =
            tokio::join!(maintenance.shutdown(), self.pool.settle_cleanup());
        if let Err(error) = cleanup {
            tracing::error!(
                %error,
                message = REMOTE_CLEANUP_FAIL_CLOSED_MESSAGE,
                "broker cleanup failed closed"
            );
            std::future::pending::<()>().await;
        }
        if credential_retirement && !retirement_settled {
            std::future::pending::<()>().await;
        }
        let result = shutdown_result(cause, maintenance);
        drop(self);
        drop(retirement_lock);
        result
    }
}
