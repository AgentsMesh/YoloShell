use super::*;

impl Host {
    pub(crate) async fn finish_or_retain_planned_resume(
        &self,
        manifest: TerminalManifest,
        transaction: &ResumeTransaction,
    ) -> std::result::Result<TerminalView, SessionResumeFailure> {
        let mut failure_manifest = manifest.clone();
        let runtime = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id))
            .map_err(|error| self.resume_transaction_failure(&mut failure_manifest, error))?;
        if runtime.bootstrap_delivery == BootstrapDelivery::Survivor {
            return self
                .retain_planned_resume_survivor(manifest, transaction, runtime)
                .await;
        }
        self.finish_planned_resume(manifest, transaction).await
    }

    pub(crate) async fn retain_planned_resume_survivor(
        &self,
        mut manifest: TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: RuntimeRecord,
    ) -> std::result::Result<TerminalView, SessionResumeFailure> {
        self.validate_planned_runtime(&manifest, transaction, &runtime)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.require_rpc_custody(&runtime)
            .await
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        let authenticated =
            crate::worker::authenticated_runtime(&runtime, self.controller_generation)
                .await
                .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.validate_planned_runtime(&manifest, transaction, &authenticated)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        let current = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id))
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        if authenticated != current {
            return Err(self
                .resume_transaction_failure(&mut manifest, RESUME_AUTHENTICATED_RUNTIME_MISMATCH));
        }
        manifest.recovery_state = RecoveryState::Uncertain;
        manifest.updated_at_ms = now_ms();
        self.persist_resume_manifest(&manifest).await?;
        self.park_resume_survivor(transaction);
        Ok(self.view_for_manifest(manifest).await)
    }
}
