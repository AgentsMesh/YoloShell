pub(crate) const CREATION_CHECKPOINT_LAUNCH_LABEL: &str = "creation checkpoint worker launch";
pub(crate) const CREATION_CHECKPOINT_RUNTIME_LABEL: &str = "creation checkpoint runtime record";

impl Host {
    pub(crate) fn reconcile_pre_runtime_checkpoint(
        &self,
        binding: &CreationBinding,
        planned: &CreationLaunchIdentity,
    ) -> std::result::Result<bool, ErrorBody> {
        let runtime_before = self.creation_checkpoint_runtime(binding.result.terminal_id)?;
        let before = if runtime_before.is_some() {
            read_creation_evidence(
                &self.paths.worker_launch(binding.result.terminal_id),
                CREATION_CHECKPOINT_LAUNCH_LABEL,
            )
            .map_err(|_| creation_launch_uncertain())?
        } else {
            self.creation_checkpoint_launch(binding, planned)?
        };
        if let Some(runtime) = runtime_before.as_ref() {
            return self.reconcile_retained_checkpoint_runtime(
                binding,
                planned,
                before.as_ref(),
                runtime,
            );
        }
        #[cfg(test)]
        creation_checkpoint_test_hook::run().map_err(|_| creation_launch_uncertain())?;
        let confirmed = self.creation_checkpoint_launch(binding, planned)?;
        if before != confirmed {
            return Err(creation_launch_uncertain());
        }
        let policy = match before.as_ref() {
            Some(_) => crate::worker::WorkerStartupCheckpointPolicy::SettlePresent,
            None => crate::worker::WorkerStartupCheckpointPolicy::RequireAbsent,
        };
        let requested = match before.as_ref() {
            Some(launch) => launch.clone(),
            None => self
                .planned_creation_worker_launch(binding, planned)
                .map_err(|_| creation_launch_uncertain())?,
        };
        let preparation_required = before
            .as_ref()
            .map(|launch| {
                crate::worker::worker_startup_checkpoint_replay_disposition(&self.paths, launch)
            })
            .transpose()
            .map_err(|_| creation_launch_uncertain())?
            == Some(crate::worker::WorkerStartupCheckpointReplayDisposition::PreparationRequired);
        let settlement = if preparation_required {
            crate::worker::WorkerStartupCheckpointSettlement::AlreadySettled
        } else {
            crate::worker::settle_worker_startup_checkpoint_for_launch(
                &self.paths,
                &requested,
                policy,
            )
            .map_err(|_| creation_launch_uncertain())?
        };
        if before.is_none()
            && settlement != crate::worker::WorkerStartupCheckpointSettlement::Absent
        {
            return Err(creation_launch_uncertain());
        }
        if self
            .creation_checkpoint_runtime(binding.result.terminal_id)?
            .is_some()
        {
            return Ok(false);
        }
        let after = self.creation_checkpoint_launch(binding, planned)?;
        if before != after {
            return Err(creation_launch_uncertain());
        }
        if let Some(launch) = after.as_ref() {
            let replayable = self
                .creation_launch_controller_is_replayable(launch)
                .map_err(|_| creation_launch_uncertain())?;
            if !replayable {
                return Err(creation_runtime_mismatch());
            }
        }
        Ok(true)
    }

    pub(crate) fn reconcile_retained_checkpoint_runtime(
        &self,
        binding: &CreationBinding,
        planned: &CreationLaunchIdentity,
        launch_before: Option<&WorkerLaunchSpec>,
        runtime_before: &RuntimeRecord,
    ) -> std::result::Result<bool, ErrorBody> {
        let launch_identity =
            classify_creation_launch(launch_before, binding, planned, &self.paths);
        let runtime_identity = classify_creation_runtime(
            Some(runtime_before),
            launch_before,
            binding,
            planned,
            &self.paths,
        );
        let Some(launch_before) = launch_before else {
            return Ok(false);
        };
        if launch_identity != CreationEvidenceIdentity::Current
            || runtime_identity != CreationEvidenceIdentity::Previous
        {
            return Ok(false);
        }
        #[cfg(test)]
        creation_checkpoint_test_hook::run().map_err(|_| creation_launch_uncertain())?;
        let launch_confirmed = self.creation_checkpoint_launch(binding, planned)?;
        let runtime_confirmed = self.creation_checkpoint_runtime(binding.result.terminal_id)?;
        if launch_confirmed.as_ref() != Some(launch_before)
            || runtime_confirmed.as_ref() != Some(runtime_before)
        {
            return Err(creation_launch_uncertain());
        }
        crate::worker::prepare_worker_startup_checkpoint_for_launch(
            &self.paths,
            launch_before,
            Some(runtime_before),
        )
        .map_err(|_| creation_launch_uncertain())?;
        let launch_prepared = self.creation_checkpoint_launch(binding, planned)?;
        let runtime_prepared = self.creation_checkpoint_runtime(binding.result.terminal_id)?;
        if launch_prepared.as_ref() != Some(launch_before)
            || runtime_prepared.as_ref() != Some(runtime_before)
        {
            return Err(creation_launch_uncertain());
        }
        self.require_retained_checkpoint_authority(launch_before, runtime_before)?;
        if !self
            .creation_launch_controller_is_replayable(launch_before)
            .map_err(|_| creation_launch_uncertain())?
        {
            return Err(creation_runtime_mismatch());
        }
        Ok(true)
    }

    pub(crate) fn require_retained_checkpoint_authority(
        &self,
        launch: &WorkerLaunchSpec,
        runtime: &RuntimeRecord,
    ) -> std::result::Result<(), ErrorBody> {
        let authorized = crate::worker::worker_startup_checkpoint_authorizes_retained_runtime(
            &self.paths,
            launch,
            runtime,
        )
        .map_err(|_| creation_launch_uncertain())?;
        if authorized {
            Ok(())
        } else {
            Err(creation_launch_uncertain())
        }
    }

    pub(crate) fn creation_checkpoint_runtime(
        &self,
        terminal_id: Uuid,
    ) -> std::result::Result<Option<RuntimeRecord>, ErrorBody> {
        read_creation_evidence(
            &self.paths.runtime_record(terminal_id),
            CREATION_CHECKPOINT_RUNTIME_LABEL,
        )
        .map_err(|_| creation_launch_uncertain())
    }

    pub(crate) fn creation_checkpoint_launch(
        &self,
        binding: &CreationBinding,
        planned: &CreationLaunchIdentity,
    ) -> std::result::Result<Option<WorkerLaunchSpec>, ErrorBody> {
        let launch = read_creation_evidence(
            &self.paths.worker_launch(binding.result.terminal_id),
            CREATION_CHECKPOINT_LAUNCH_LABEL,
        )
        .map_err(|_| creation_launch_uncertain())?;
        if let Some(launch) = launch.as_ref() {
            if !launch_matches_binding(launch, binding, planned, &self.paths) {
                return Err(creation_runtime_mismatch());
            }
        }
        Ok(launch)
    }
}
