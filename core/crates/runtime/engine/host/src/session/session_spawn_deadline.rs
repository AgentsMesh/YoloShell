pub(super) const WORKER_START_DEADLINE_EXPIRED: &str = "worker start deadline expired before spawn";
const WORKER_REAPER_HANDOFF_FAILED: &str = "worker reaper stopped before accepting process";
const WORKER_REAPER_FALLBACK_FAILED: &str = "fallback worker reaper failed";

impl Host {
    pub(super) async fn spawn_published_worker_before(
        &self,
        manifest: &TerminalManifest,
        launch: &WorkerLaunchSpec,
        failure_policy: UnstartedLaunchPolicy,
        deadline: HostOperationDeadline,
    ) -> Result<WorkerStart> {
        anyhow::ensure!(
            launch.terminal_id == manifest.terminal_id,
            PUBLISHED_LAUNCH_SESSION_MISMATCH
        );
        let arguments = crate::cli::ProcessRole::Host.prepend([
            crate::cli::constants::SESSION_WORKER_COMMAND.into(),
            crate::cli::constants::HOME_ARGUMENT.into(),
            self.paths.home().as_os_str().to_owned(),
            crate::cli::constants::TERMINAL_ID_ARGUMENT.into(),
            manifest.terminal_id.to_string().into(),
        ]);
        self.require_worker_spawn_budget_before_checkpoint(launch, failure_policy, deadline)?;
        let preparation = self.prepare_worker_startup_checkpoint_before_spawn(manifest, launch)?;
        let failure_policy = match preparation {
            crate::worker::WorkerStartupCheckpointPreparation::Absent => failure_policy,
            crate::worker::WorkerStartupCheckpointPreparation::Prepared
            | crate::worker::WorkerStartupCheckpointPreparation::AlreadyPrepared => {
                UnstartedLaunchPolicy::Preserve
            }
        };
        self.require_worker_spawn_budget_or_apply(manifest.terminal_id, failure_policy, deadline)?;
        let reaper = match self.prepare_worker_reaper(manifest, failure_policy) {
            Ok(reaper) => reaper,
            Err(error) => return Err(error),
        };
        let worker_process = match self.spawn_session_worker_before(&arguments, deadline) {
            Ok(process) => process,
            Err(error) => {
                self.apply_unstarted_launch_policy(manifest.terminal_id, failure_policy)?;
                return Err(error);
            }
        };
        if let Err(error) = reaper.send(worker_process) {
            let process = error.0;
            let process_id = process.id();
            let fallback = process.reap_in_background(format!(
                "{WORKER_REAPER_THREAD_PREFIX}-{}",
                manifest.terminal_id
            ));
            let detail = match process_id {
                Ok(process_id) => format!("{WORKER_REAPER_HANDOFF_FAILED} {process_id}"),
                Err(id_error) => format!("{WORKER_REAPER_HANDOFF_FAILED}: {id_error:#}"),
            };
            let detail = fallback
                .err()
                .map(|error| format!("{detail}; {WORKER_REAPER_FALLBACK_FAILED}: {error:#}"))
                .unwrap_or(detail);
            return Ok(WorkerStart::Uncertain(anyhow::anyhow!("{detail}")));
        }
        self.await_worker_start_before(manifest, launch.runtime_epoch, launch.worker_id, deadline)
            .await
    }

    fn prepare_worker_reaper(
        &self,
        manifest: &TerminalManifest,
        policy: UnstartedLaunchPolicy,
    ) -> Result<std::sync::mpsc::SyncSender<crate::process::command_worker::WorkerProcess>> {
        spawn_worker_reaper(manifest.terminal_id).or_else(|error| {
            self.apply_unstarted_launch_policy(manifest.terminal_id, policy)?;
            Err(error)
        })
    }

    fn require_worker_spawn_budget_or_apply(
        &self,
        terminal_id: Uuid,
        policy: UnstartedLaunchPolicy,
        deadline: HostOperationDeadline,
    ) -> Result<()> {
        if let Err(error) = require_worker_spawn_budget(deadline) {
            self.apply_unstarted_launch_policy(terminal_id, policy)?;
            return Err(error);
        }
        Ok(())
    }

    fn require_worker_spawn_budget_before_checkpoint(
        &self,
        launch: &WorkerLaunchSpec,
        policy: UnstartedLaunchPolicy,
        deadline: HostOperationDeadline,
    ) -> Result<()> {
        let Err(error) = require_worker_spawn_budget(deadline) else {
            return Ok(());
        };
        let policy = match policy {
            UnstartedLaunchPolicy::Preserve => UnstartedLaunchPolicy::Preserve,
            UnstartedLaunchPolicy::Retire => {
                match crate::worker::worker_startup_checkpoint_replay_disposition(
                    &self.paths,
                    launch,
                ) {
                    Ok(crate::worker::WorkerStartupCheckpointReplayDisposition::Absent) => {
                        UnstartedLaunchPolicy::Retire
                    }
                    Ok(_) | Err(_) => UnstartedLaunchPolicy::Preserve,
                }
            }
        };
        self.apply_unstarted_launch_policy(launch.terminal_id, policy)?;
        Err(error)
    }

    pub(super) async fn await_worker_start_before(
        &self,
        manifest: &TerminalManifest,
        runtime_epoch: Uuid,
        worker_id: Uuid,
        deadline: HostOperationDeadline,
    ) -> Result<WorkerStart> {
        while !deadline.is_elapsed() {
            if let Some(result) = self
                .worker_start_evidence(manifest, runtime_epoch, worker_id, deadline)
                .await?
            {
                return Ok(result);
            }
            deadline
                .wait_for_poll(self.worker_ready_poll_interval)
                .await;
        }
        self.classify_worker_start_timeout(manifest, runtime_epoch, worker_id)
    }

    async fn worker_start_evidence(
        &self,
        manifest: &TerminalManifest,
        runtime_epoch: Uuid,
        worker_id: Uuid,
        deadline: HostOperationDeadline,
    ) -> Result<Option<WorkerStart>> {
        let Ok(record) = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id))
        else {
            return Ok(None);
        };
        if record.runtime_epoch != runtime_epoch || record.worker_id != worker_id {
            return Ok(None);
        }
        match bootstrap_worker_start(record.bootstrap_delivery)? {
            BootstrapWorkerAction::AwaitFinal => {
                let _ = deadline
                    .timeout(self.ensure_secondary_custody(&record))
                    .await;
                Ok(None)
            }
            BootstrapWorkerAction::Uncertain(error) => Ok(Some(WorkerStart::Uncertain(error))),
            BootstrapWorkerAction::Authenticate => {
                let authenticated = deadline
                    .timeout(self.custodied_worker_stream(&record))
                    .await;
                Ok(authenticated
                    .and_then(Result::ok)
                    .map(|_| WorkerStart::Ready))
            }
        }
    }
}

fn require_worker_spawn_budget(deadline: HostOperationDeadline) -> Result<()> {
    anyhow::ensure!(!deadline.is_elapsed(), WORKER_START_DEADLINE_EXPIRED);
    Ok(())
}
