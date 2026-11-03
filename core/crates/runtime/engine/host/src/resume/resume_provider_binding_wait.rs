use super::*;

pub(crate) const RESUME_PROVIDER_BINDING_TIMEOUT: Duration =
    crate::agent::constants::DETECTION_INTERVAL
        .saturating_add(crate::agent::constants::DETECTION_INTERVAL)
        .saturating_add(crate::session_timing::PROCESS_CAPABILITY_EXIT_EVENT_CONVERGENCE_TIMEOUT);
pub(crate) const RESUME_PROVIDER_BINDING_POLL_INTERVAL: Duration = Duration::from_millis(25);
pub(crate) const RESUME_PROVIDER_BINDING_PENDING: &str =
    "provider resume binding is not yet authoritative";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProviderBindingObservation {
    Missing,
    Pending,
    Rejected,
    Candidate(AgentBinding),
}

impl Host {
    pub(crate) async fn await_authoritative_resume_binding(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> ProviderCompletionResult<Option<ResumeTransaction>> {
        if !matches!(&transaction.target, ResumeTarget::Provider { .. }) {
            return Ok(None);
        }
        let deadline = HostOperationDeadline::after(self.resume_provider_binding_timeout);
        let mut accepted_runtime = runtime.clone();
        loop {
            let observation = match self
                .capture_resume_provider_snapshot(manifest, transaction, &accepted_runtime)
                .map_err(ProviderCompletionError::contradiction)?
            {
                Some((current, observation)) => {
                    accepted_runtime = current;
                    observation
                }
                None => ProviderBindingObservation::Pending,
            };
            match &observation {
                ProviderBindingObservation::Rejected => {
                    return Err(ProviderCompletionError::contradiction(anyhow::anyhow!(
                        RESUME_PROVIDER_BINDING_MISMATCH
                    )));
                }
                ProviderBindingObservation::Candidate(binding) if !deadline.is_elapsed() => {
                    match self
                        .provider_runtime_membership(
                            transaction,
                            &accepted_runtime,
                            binding,
                            deadline,
                        )
                        .await
                    {
                        ProviderRuntimeMembership::Verified(observed_at_ms) => {
                            if let Some(authority) = self
                                .settle_verified_provider_candidate(
                                    manifest,
                                    transaction,
                                    &mut accepted_runtime,
                                    binding,
                                    observed_at_ms,
                                )
                                .map_err(ProviderCompletionError::contradiction)?
                            {
                                return Ok(Some(authority));
                            }
                        }
                        ProviderRuntimeMembership::Contradicted => {
                            return Err(ProviderCompletionError::contradiction(anyhow::anyhow!(
                                RESUME_PROVIDER_BINDING_MISMATCH
                            )));
                        }
                        ProviderRuntimeMembership::NotObserved => {}
                    }
                }
                ProviderBindingObservation::Missing
                | ProviderBindingObservation::Pending
                | ProviderBindingObservation::Candidate(_) => {}
            }
            if deadline.is_elapsed() {
                return provider_binding_timeout(&observation);
            }
            deadline
                .wait_for_poll(self.resume_provider_binding_poll_interval)
                .await;
        }
    }

    pub(crate) fn observe_resume_provider_binding(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<ProviderBindingObservation> {
        match self.capture_resume_provider_snapshot(manifest, transaction, runtime)? {
            Some((_, observation)) => Ok(observation),
            None => Ok(ProviderBindingObservation::Pending),
        }
    }

    pub(crate) fn settle_verified_provider_candidate(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        accepted_runtime: &mut RuntimeRecord,
        binding: &AgentBinding,
        observed_at_ms: i64,
    ) -> Result<Option<ResumeTransaction>> {
        let Some(_publication_lock) = self
            .paths
            .try_acquire_session_lifecycle_lock(manifest.terminal_id)?
        else {
            return Ok(None);
        };
        let current = self.read_namespace_json::<RuntimeRecord>(
            &self.paths.runtime_record(manifest.terminal_id),
        )?;
        self.validate_resume_runtime_update(manifest, transaction, accepted_runtime, &current)?;
        *accepted_runtime = current;
        let current_binding = self.load_resume_agent_binding(manifest, accepted_runtime)?;
        if current_binding.as_ref() != Some(binding) {
            return Ok(None);
        }
        if !self.provider_startup_receipt_ready(accepted_runtime, binding)? {
            return Ok(None);
        }
        let authority = ProviderResumeAuthority::new(
            transaction,
            accepted_runtime,
            binding.clone(),
            observed_at_ms,
        )?;
        self.record_provider_completion(transaction, authority, manifest, accepted_runtime)
            .map(Some)
    }
}

pub(crate) fn provider_binding_timeout(
    observation: &ProviderBindingObservation,
) -> ProviderCompletionResult<Option<ResumeTransaction>> {
    let detail = if matches!(observation, ProviderBindingObservation::Missing) {
        RESUME_PROVIDER_BINDING_MISSING
    } else {
        RESUME_PROVIDER_BINDING_PENDING
    };
    Err(ProviderCompletionError::pending(detail))
}
