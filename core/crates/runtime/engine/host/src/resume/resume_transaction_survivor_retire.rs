use super::*;

#[cfg(any(test, feature = "failed-rearm-test-hooks"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/unit/resume/resume_transaction_survivor_retire_test_hook.rs"]
pub(crate) mod resume_terminal_rearm_test_hook;

impl Host {
    pub(crate) async fn rearm_dead_resume_terminal(
        &self,
        manifest: &mut TerminalManifest,
        transaction: &ResumeTransaction,
    ) -> std::result::Result<Option<ResumeTransaction>, SessionResumeFailure> {
        let mut durable = self
            .load_durable_manifest(manifest.terminal_id)
            .map_err(|error| self.resume_transaction_failure(manifest, error))?;
        if durable.recovery_state == transaction.target.completed_state() {
            *manifest = durable;
            return Ok(None);
        }
        let Some(runtime) = self
            .dead_resume_terminal_evidence(&durable, transaction)
            .map_err(|error| self.resume_transaction_failure(manifest, error))?
        else {
            return Ok(None);
        };
        if socket_may_be_serving(&runtime.socket_path).await {
            self.persist_recovery_state(&mut durable, RecoveryState::Uncertain)
                .await;
            *manifest = durable;
            return Err(ErrorBody::conflict(RESUME_SOCKET_ACTIVE).into());
        }
        self.roll_resume_tombstone(transaction, runtime.runtime_epoch)
            .map_err(|error| self.resume_transaction_failure(manifest, error))?;
        #[cfg(any(test, feature = "failed-rearm-test-hooks"))]
        resume_terminal_rearm_test_hook::run(
            resume_terminal_rearm_test_hook::Boundary::TombstoneRolled,
        )
        .map_err(internal)?;
        self.remove_completed_resume_launch(&durable, transaction)?;
        #[cfg(any(test, feature = "failed-rearm-test-hooks"))]
        resume_terminal_rearm_test_hook::run(
            resume_terminal_rearm_test_hook::Boundary::PreviousLaunchRemoved,
        )
        .map_err(internal)?;
        durable.recovery_state = RecoveryState::Uncertain;
        durable.updated_at_ms = now_ms();
        self.persist_resume_manifest(&durable).await?;
        #[cfg(any(test, feature = "failed-rearm-test-hooks"))]
        resume_terminal_rearm_test_hook::run(
            resume_terminal_rearm_test_hook::Boundary::UncertainManifestPersisted,
        )
        .map_err(internal)?;
        *manifest = durable;
        let replacement = self
            .prepare_terminal_rearm(manifest, transaction, runtime.runtime_epoch)
            .map_err(|error| self.resume_transaction_failure(manifest, error))?;
        self.validate_terminal_rearm_evidence(manifest, transaction, &runtime)
            .map_err(|error| self.resume_transaction_failure(manifest, error))?;
        self.publish_resume_transaction(&replacement)
            .map_err(internal)?;
        #[cfg(any(test, feature = "failed-rearm-test-hooks"))]
        resume_terminal_rearm_test_hook::run(
            resume_terminal_rearm_test_hook::Boundary::ReplacementTransactionPublished,
        )
        .map_err(internal)?;
        self.publish_worker_launch(&replacement.launch)
            .map_err(internal)?;
        #[cfg(any(test, feature = "failed-rearm-test-hooks"))]
        resume_terminal_rearm_test_hook::run(
            resume_terminal_rearm_test_hook::Boundary::ReplacementLaunchPublished,
        )
        .map_err(internal)?;
        Ok(Some(replacement))
    }

    pub(crate) fn prepare_terminal_rearm(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        retired_runtime_epoch: Uuid,
    ) -> Result<ResumeTransaction> {
        let resume = match &transaction.target {
            ResumeTarget::Provider { binding, .. } => Some((binding, transaction.intent)),
            ResumeTarget::Shell { .. } => None,
        };
        let launch = self.prepare_worker_launch(
            manifest,
            transaction.target.launch_command(manifest),
            resume,
            None,
        )?;
        let replacement = ResumeTransaction::new(
            retired_runtime_epoch,
            transaction.intent,
            transaction.target.clone(),
            &launch,
        )?;
        self.validate_transaction_launch(manifest, &replacement, &launch)?;
        Ok(replacement)
    }

    pub(crate) fn validate_terminal_rearm_evidence(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        let durable = self.load_durable_manifest(manifest.terminal_id)?;
        anyhow::ensure!(
            serde_json::to_value(&durable)? == serde_json::to_value(manifest)?,
            RESUME_MANIFEST_MISMATCH
        );
        let current = self
            .load_resume_transaction(manifest.terminal_id)?
            .context(RESUME_TRANSACTION_MISSING)?;
        anyhow::ensure!(current == *transaction, RESUME_TRANSACTION_CONFLICT);
        let current_runtime = self
            .dead_resume_terminal_evidence(manifest, transaction)?
            .context(RESUME_PREVIOUS_RUNTIME_UNRESOLVED)?;
        anyhow::ensure!(current_runtime == *runtime, RESUME_EVIDENCE_MISMATCH);
        anyhow::ensure!(
            self.namespace
                .file_is_missing(&self.paths.worker_launch(manifest.terminal_id))?,
            RESUME_LAUNCH_WITHOUT_TRANSACTION
        );
        Ok(())
    }

    pub(crate) fn roll_resume_tombstone(
        &self,
        transaction: &ResumeTransaction,
        retired_runtime_epoch: Uuid,
    ) -> Result<()> {
        if transaction.intent != ResumeIntent::Explicit {
            return Ok(());
        }
        let path = self.paths.terminate_tombstone(transaction.terminal_id);
        if self.namespace.file_is_missing(&path)? {
            return Ok(());
        }
        let mut tombstone =
            self.read_namespace_json::<crate::model::TerminationTombstone>(&path)?;
        if tombstone
            .validate_for(transaction.terminal_id, retired_runtime_epoch)
            .is_ok()
        {
            return Ok(());
        }
        tombstone
            .validate_for(transaction.terminal_id, transaction.previous_runtime_epoch)
            .map_err(|error| anyhow::anyhow!("{RESUME_TOMBSTONE_MISMATCH}: {error}"))?;
        tombstone.runtime_epoch = retired_runtime_epoch;
        self.write_namespace_json(&path, &tombstone)
    }
}
