pub(crate) enum WorkerStart {
    Ready,
    Uncertain(anyhow::Error),
}

pub(crate) fn spawn_worker_reaper(
    terminal_id: Uuid,
) -> Result<std::sync::mpsc::SyncSender<crate::process::command_worker::WorkerProcess>> {
    let (sender, receiver) =
        std::sync::mpsc::sync_channel::<crate::process::command_worker::WorkerProcess>(
            WORKER_REAPER_CHANNEL_CAPACITY,
        );
    std::thread::Builder::new()
        .name(format!("{WORKER_REAPER_THREAD_PREFIX}-{terminal_id}"))
        .spawn(move || {
            while let Ok(mut process) = receiver.recv() {
                let _ = process.wait();
            }
        })
        .context("spawn worker reaper")?;
    Ok(sender)
}

impl Host {
    pub(crate) async fn mark_worker_start_uncertain(
        &self,
        manifest: &mut TerminalManifest,
        error: &anyhow::Error,
    ) {
        tracing::error!(
            terminal_id = %manifest.terminal_id,
            %error,
            message = WORKER_START_UNCERTAIN_LOG,
        );
        self.persist_recovery_state(manifest, RecoveryState::Uncertain)
            .await;
    }

    pub(crate) async fn mark_creation_worker_start_uncertain_before(
        &self,
        manifest: &mut TerminalManifest,
        error: &anyhow::Error,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<(), ErrorBody> {
        tracing::error!(
            terminal_id = %manifest.terminal_id,
            %error,
            message = WORKER_START_UNCERTAIN_LOG,
        );
        require_creation_budget(deadline)?;
        let Some(candidate) = self.prepare_recovery_state(manifest, RecoveryState::Uncertain)
        else {
            return Err(creation_launch_uncertain());
        };
        self.write_creation_json_before(
            &self.paths.manifest(candidate.terminal_id),
            &candidate,
            deadline,
        )?;
        *manifest = candidate;
        self.catalog_upsert_before(manifest, deadline)
            .await
            .map_err(creation_publication_error)?;
        #[cfg(test)]
        self.notify_recovery_publication(manifest);
        Ok(())
    }
}
