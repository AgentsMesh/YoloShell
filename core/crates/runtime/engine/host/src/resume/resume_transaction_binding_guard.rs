use super::*;

impl Host {
    pub(crate) fn stage_resume_binding_disposition(
        &self,
        transaction: &ResumeTransaction,
    ) -> Result<()> {
        let guard = yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
            &self.paths,
            &self.namespace,
            transaction.terminal_id,
        )?;
        self.stage_resume_binding_disposition_guarded(transaction, &guard)
    }

    pub(crate) fn stage_resume_binding_disposition_after_lifecycle_lock(
        &self,
        transaction: &ResumeTransaction,
    ) -> Result<()> {
        let guard =
            yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_after_lifecycle_lock(
                &self.paths,
                &self.namespace,
                transaction.terminal_id,
            )?;
        self.stage_resume_binding_disposition_guarded(transaction, &guard)
    }

    pub(crate) fn stage_resume_binding_disposition_with_replay_launch(
        &self,
        transaction: &ResumeTransaction,
    ) -> Result<WorkerLaunchSpec> {
        let guard = yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
            &self.paths,
            &self.namespace,
            transaction.terminal_id,
        )?;
        let persisted = guard
            .read_optional_with_identity::<ResumeTransaction>(
                &self.paths.resume_transaction(transaction.terminal_id),
            )?
            .map(|(value, _)| value);
        anyhow::ensure!(
            persisted.as_ref() == Some(transaction),
            RESUME_EVIDENCE_MISMATCH
        );
        let path = self.paths.worker_launch(transaction.terminal_id);
        let launch = match guard.read_optional_with_identity::<WorkerLaunchSpec>(&path)? {
            Some((launch, _)) => launch,
            None => {
                guard.write_json_expected_with_commit_fence(&path, None, &transaction.launch)?;
                transaction.launch.clone()
            }
        };
        transaction.validate_launch(&launch)?;
        self.stage_resume_binding_disposition_guarded(transaction, &guard)?;
        Ok(launch)
    }

    pub(crate) fn stage_resume_binding_disposition_guarded(
        &self,
        transaction: &ResumeTransaction,
        guard: &yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>,
    ) -> Result<()> {
        guard.require_no_pending_intent()?;
        let Some((_, previous, codex_authority)) = binding_to_clear(transaction) else {
            return self.ensure_binding_marker_missing(transaction.terminal_id);
        };
        let path = self.paths.resume_binding_clear(transaction.terminal_id);
        let marker = match guard
            .read_optional_with_identity::<ResumeBindingClearMarker>(&path)?
            .map(|(value, _)| value)
        {
            Some(marker) if marker.validate_for(transaction).is_ok() => marker,
            Some(marker) => self.roll_resume_binding_marker(marker, transaction)?,
            None => {
                let marker = binding_clear_marker(transaction, ResumeBindingClearPhase::Pending)?;
                guard.write_json_expected_with_commit_fence(&path, None, &marker)?;
                marker
            }
        };
        marker.validate_for(transaction)?;
        if marker.phase == ResumeBindingClearPhase::Cleared {
            return Ok(());
        }
        self.revoke_resume_codex_authority(guard, transaction, &marker, codex_authority.as_ref())?;
        self.agent_binding_store(transaction.terminal_id)
            .revoke_native_authorities_with_guard(guard, || guard.require_no_pending_intent())?;
        self.clear_previous_binding(transaction.terminal_id, previous.as_ref())?;
        let cleared = binding_clear_marker(transaction, ResumeBindingClearPhase::Cleared)?;
        self.replace_resume_binding_marker(&marker, &cleared)?;
        cleared.validate_for(transaction)?;
        guard.require_no_pending_intent()
    }
}
