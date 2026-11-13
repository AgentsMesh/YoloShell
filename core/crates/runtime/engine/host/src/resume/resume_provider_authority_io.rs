use super::*;

impl Host {
    pub(crate) fn record_provider_completion(
        &self,
        expected: &ResumeTransaction,
        authority: ProviderResumeAuthority,
        manifest: &TerminalManifest,
        accepted_runtime: &RuntimeRecord,
    ) -> Result<ResumeTransaction> {
        authority.validate_for(expected)?;
        authority.validate_runtime(accepted_runtime)?;
        let path = self.paths.resume_transaction(expected.terminal_id);
        let (mut current, identity) = self
            .namespace
            .read_json_with_identity::<ResumeTransaction>(&path)?;
        current.validate_for(expected.terminal_id)?;
        anyhow::ensure!(
            current.transaction_digest == expected.transaction_digest,
            RESUME_TRANSACTION_DIGEST_INVALID
        );
        if let Some(existing) = current.provider_completion.as_ref() {
            anyhow::ensure!(existing == &authority, RESUME_PROVIDER_AUTHORITY_INVALID);
            return Ok(current);
        }
        current.provider_completion = Some(authority.clone());
        current.validate_for(expected.terminal_id)?;
        self.namespace.write_json_expected_with_commit_fence(
            &path,
            Some(identity),
            &current,
            || {
                self.validate_provider_completion_commit(
                    manifest,
                    &current,
                    accepted_runtime,
                    &authority,
                )
            },
        )?;
        Ok(current)
    }

    pub(crate) fn validate_provider_completion_commit(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        accepted_runtime: &RuntimeRecord,
        authority: &ProviderResumeAuthority,
    ) -> Result<()> {
        let runtime = self.read_namespace_json::<RuntimeRecord>(
            &self.paths.runtime_record(transaction.terminal_id),
        )?;
        self.validate_resume_runtime_update(manifest, transaction, accepted_runtime, &runtime)?;
        authority.validate_runtime(&runtime)?;
        let binding = self
            .load_resume_agent_binding(manifest, &runtime)?
            .context(RESUME_PROVIDER_BINDING_MISSING)?;
        anyhow::ensure!(
            provider_binding_commit_matches(&authority.binding, &binding),
            RESUME_PROVIDER_BINDING_MISMATCH
        );
        let identity = binding
            .process_pid
            .zip(binding.process_start_time)
            .context(RESUME_PROVIDER_BINDING_MISMATCH)?;
        anyhow::ensure!(
            process_identity_is_verified_live(identity.0, identity.1),
            RESUME_PROVIDER_BINDING_MISMATCH
        );
        anyhow::ensure!(
            self.provider_startup_receipt_ready(&runtime, &binding)?,
            RESUME_PROVIDER_STARTUP_RECEIPT_MISSING
        );
        Ok(())
    }
}

pub(crate) fn provider_binding_commit_matches(
    observed: &AgentBinding,
    current: &AgentBinding,
) -> bool {
    provider_binding_stable_fields_match(observed, current)
        && current.process_pid == observed.process_pid
        && current.process_start_time == observed.process_start_time
        && current.updated_at_ms >= observed.updated_at_ms
        && current.recovery_availability == crate::agent::AgentRecoveryAvailability::Available
        && current.state.is_active()
}
