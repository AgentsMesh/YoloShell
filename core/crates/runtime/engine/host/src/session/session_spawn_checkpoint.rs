const WORKER_CHECKPOINT_LAUNCH_LABEL: &str = "worker checkpoint successor launch";
const WORKER_CHECKPOINT_RUNTIME_LABEL: &str = "worker checkpoint predecessor runtime";
const WORKER_CHECKPOINT_LAUNCH_MISSING: &str = "worker checkpoint successor launch is missing";
const WORKER_CHECKPOINT_LAUNCH_MISMATCH: &str = "worker checkpoint successor launch changed";
const WORKER_CHECKPOINT_RUNTIME_CHANGED: &str = "worker checkpoint predecessor runtime changed";

impl Host {
    pub(super) fn prepare_worker_startup_checkpoint_before_spawn(
        &self,
        manifest: &TerminalManifest,
        successor: &WorkerLaunchSpec,
    ) -> Result<crate::worker::WorkerStartupCheckpointPreparation> {
        anyhow::ensure!(
            successor.terminal_id == manifest.terminal_id,
            PUBLISHED_LAUNCH_SESSION_MISMATCH
        );
        let launch_before = self.read_checkpoint_successor(manifest.terminal_id)?;
        anyhow::ensure!(
            launch_before.as_ref() == Some(successor),
            WORKER_CHECKPOINT_LAUNCH_MISMATCH
        );
        let runtime_before = self.read_checkpoint_predecessor(manifest.terminal_id)?;
        #[cfg(test)]
        creation_checkpoint_test_hook::run()?;
        let launch_confirmed = self.read_checkpoint_successor(manifest.terminal_id)?;
        let runtime_confirmed = self.read_checkpoint_predecessor(manifest.terminal_id)?;
        anyhow::ensure!(
            launch_confirmed == launch_before,
            WORKER_CHECKPOINT_LAUNCH_MISMATCH
        );
        anyhow::ensure!(
            runtime_confirmed == runtime_before,
            WORKER_CHECKPOINT_RUNTIME_CHANGED
        );
        let preparation = crate::worker::prepare_worker_startup_checkpoint_for_launch(
            &self.paths,
            successor,
            runtime_before.as_ref(),
        )?;
        anyhow::ensure!(
            self.read_checkpoint_successor(manifest.terminal_id)? == launch_before,
            WORKER_CHECKPOINT_LAUNCH_MISMATCH
        );
        anyhow::ensure!(
            self.read_checkpoint_predecessor(manifest.terminal_id)? == runtime_before,
            WORKER_CHECKPOINT_RUNTIME_CHANGED
        );
        Ok(preparation)
    }

    fn read_checkpoint_successor(&self, terminal_id: Uuid) -> Result<Option<WorkerLaunchSpec>> {
        read_creation_evidence(
            &self.paths.worker_launch(terminal_id),
            WORKER_CHECKPOINT_LAUNCH_LABEL,
        )
        .and_then(|launch| {
            anyhow::ensure!(launch.is_some(), WORKER_CHECKPOINT_LAUNCH_MISSING);
            Ok(launch)
        })
    }

    fn read_checkpoint_predecessor(&self, terminal_id: Uuid) -> Result<Option<RuntimeRecord>> {
        read_creation_evidence(
            &self.paths.runtime_record(terminal_id),
            WORKER_CHECKPOINT_RUNTIME_LABEL,
        )
    }
}
