use super::*;

impl Host {
    pub(crate) async fn finish_planned_resume(
        &self,
        mut manifest: TerminalManifest,
        transaction: &ResumeTransaction,
    ) -> std::result::Result<TerminalView, SessionResumeFailure> {
        let runtime = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id))
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.validate_initial_completion_runtime(&manifest, transaction, &runtime)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.require_rpc_custody(&runtime)
            .await
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        let authenticated =
            crate::worker::authenticated_runtime(&runtime, self.controller_generation)
                .await
                .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.validate_resume_runtime_update(&manifest, transaction, &runtime, &authenticated)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        let persisted = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id))
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.validate_resume_runtime_update(&manifest, transaction, &authenticated, &persisted)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        let transaction = match self
            .ensure_provider_completion(&manifest, transaction, &persisted)
            .await
        {
            Ok(transaction) => transaction,
            Err(error) => {
                if matches!(error, ProviderCompletionError::Pending(_)) {
                    self.settle_pending_provider_resume(&mut manifest, transaction)
                        .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
                }
                return Err(error.into_resume_failure(self, &mut manifest));
            }
        };
        let provider = self
            .completed_provider_binding(&transaction, &persisted)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.remove_completed_resume_launch(&manifest, &transaction)?;
        self.consume_resume_tombstone(&transaction)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.apply_resume_binding_disposition(&transaction)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        manifest.recovery_state = transaction.target.completed_state();
        manifest.updated_at_ms = now_ms();
        self.persist_resume_manifest(&manifest).await?;
        self.complete_resume_binding_disposition(&transaction)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.remove_resume_transaction(manifest.terminal_id)
            .map_err(internal)?;
        Ok(self.completed_resume_view(manifest, provider).await)
    }

    pub(crate) async fn finish_committed_resume(
        &self,
        mut manifest: TerminalManifest,
        transaction: &ResumeTransaction,
    ) -> std::result::Result<TerminalView, SessionResumeFailure> {
        if manifest.recovery_state != transaction.target.completed_state() {
            return Err(self.resume_transaction_failure(&mut manifest, RESUME_MANIFEST_MISMATCH));
        }
        let runtime = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id))
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.validate_committed_resume_runtime(&manifest, transaction, &runtime)
            .and_then(|()| self.validate_resume_delivery(transaction, &runtime))
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        let provider = self
            .completed_provider_binding(transaction, &runtime)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.catalog_upsert(&manifest).await.map_err(internal)?;
        self.complete_resume_binding_disposition(transaction)
            .map_err(|error| self.resume_transaction_failure(&mut manifest, error))?;
        self.remove_resume_transaction(manifest.terminal_id)
            .map_err(internal)?;
        Ok(self.completed_resume_view(manifest, provider).await)
    }

    pub(crate) fn remove_completed_resume_launch(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
    ) -> std::result::Result<(), SessionResumeFailure> {
        let path = self.paths.worker_launch(manifest.terminal_id);
        let launch = self
            .read_optional_resume_json::<WorkerLaunchSpec>(&path)
            .map_err(internal)?;
        if let Some(launch) = launch {
            self.validate_transaction_launch(manifest, transaction, &launch)
                .map_err(internal)?;
            self.namespace.remove_file(&path).map_err(internal)?;
        }
        Ok(())
    }
}
