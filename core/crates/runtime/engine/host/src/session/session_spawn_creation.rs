const CREATION_LAUNCH_REPLAY_LABEL: &str = "creation worker launch replay";
const CREATION_LAUNCH_REPLAY_UNAUTHORIZED: &str =
    "creation worker launch is not authorized for replay";

impl Host {
    pub(super) fn prepare_creation_worker_launch(
        &self,
        binding: &CreationBinding,
        planned: &CreationLaunchIdentity,
    ) -> Result<WorkerLaunchSpec> {
        let candidate = self.planned_creation_worker_launch(binding, planned)?;
        let existing = read_creation_evidence::<WorkerLaunchSpec>(
            &self.paths.worker_launch(binding.result.terminal_id),
            CREATION_LAUNCH_REPLAY_LABEL,
        )?;
        match classify_creation_launch(existing.as_ref(), binding, planned, &self.paths) {
            CreationEvidenceIdentity::Missing | CreationEvidenceIdentity::Previous => Ok(candidate),
            CreationEvidenceIdentity::Current => {
                let existing = existing.context(CREATION_LAUNCH_REPLAY_UNAUTHORIZED)?;
                anyhow::ensure!(
                    self.creation_launch_controller_is_replayable(&existing)?,
                    CREATION_LAUNCH_REPLAY_UNAUTHORIZED
                );
                Ok(existing)
            }
            CreationEvidenceIdentity::Invalid => {
                anyhow::bail!(CREATION_LAUNCH_REPLAY_UNAUTHORIZED)
            }
        }
    }

    pub(super) fn creation_launch_controller_is_replayable(
        &self,
        launch: &WorkerLaunchSpec,
    ) -> Result<bool> {
        if launch.controller_generation == self.controller_generation {
            return Ok(true);
        }
        if launch.controller_generation > self.controller_generation {
            return Ok(false);
        }
        let disposition =
            crate::worker::worker_startup_checkpoint_replay_disposition(&self.paths, launch)?;
        Ok(matches!(
            disposition,
            crate::worker::WorkerStartupCheckpointReplayDisposition::Absent
                | crate::worker::WorkerStartupCheckpointReplayDisposition::Authorized
                | crate::worker::WorkerStartupCheckpointReplayDisposition::PreparationRequired
        ))
    }

    pub(super) fn publish_creation_worker_launch_before(
        &self,
        launch: &WorkerLaunchSpec,
        deadline: HostOperationDeadline,
    ) -> Result<()> {
        self.write_creation_json_raw_before(
            &self.paths.worker_launch(launch.terminal_id),
            launch,
            deadline,
        )
    }

    pub(super) fn planned_creation_worker_launch(
        &self,
        binding: &CreationBinding,
        planned: &CreationLaunchIdentity,
    ) -> Result<WorkerLaunchSpec> {
        let manifest = self.read_namespace_json::<TerminalManifest>(
            &self.paths.manifest(binding.result.terminal_id),
        )?;
        anyhow::ensure!(
            manifest_matches_binding(&manifest, binding),
            CREATION_LAUNCH_REPLAY_UNAUTHORIZED
        );
        let mut launch = self.prepare_worker_launch(
            &manifest,
            shell_command(&binding.spec.cwd),
            None,
            Some(planned),
        )?;
        launch.initial_input = binding.spec.initial_input.clone();
        launch.context = binding.spec.context;
        Ok(launch)
    }
}
