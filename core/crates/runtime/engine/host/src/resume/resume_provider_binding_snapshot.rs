use super::*;

impl Host {
    pub(crate) fn capture_resume_provider_snapshot(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        accepted_runtime: &RuntimeRecord,
    ) -> Result<Option<(RuntimeRecord, ProviderBindingObservation)>> {
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
        let observation = self.classify_resume_provider_binding(manifest, transaction, &current)?;
        Ok(Some((current, observation)))
    }

    pub(crate) fn classify_resume_provider_binding(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<ProviderBindingObservation> {
        let ResumeTarget::Provider {
            binding: planned, ..
        } = &transaction.target
        else {
            return Ok(ProviderBindingObservation::Pending);
        };
        let store = self.agent_binding_store(manifest.terminal_id);
        if store.read_raw_in_namespace(&self.namespace)?.as_ref() == Some(planned) {
            return Ok(ProviderBindingObservation::Pending);
        }
        let Some(current) = self.load_resume_agent_binding(manifest, runtime)? else {
            return Ok(ProviderBindingObservation::Missing);
        };
        if !provider_binding_stable_fields_match(planned, &current) {
            return Ok(ProviderBindingObservation::Rejected);
        }
        let Some(identity) = current.process_pid.zip(current.process_start_time) else {
            return Ok(ProviderBindingObservation::Pending);
        };
        if Some(identity) == planned.process_pid.zip(planned.process_start_time)
            || current.recovery_availability != crate::agent::AgentRecoveryAvailability::Available
            || !current.state.is_active()
        {
            return Ok(ProviderBindingObservation::Pending);
        }
        Ok(ProviderBindingObservation::Candidate(current))
    }

    pub(crate) fn load_resume_agent_binding(
        &self,
        manifest: &TerminalManifest,
        runtime: &RuntimeRecord,
    ) -> Result<Option<AgentBinding>> {
        let store = self.agent_binding_store(manifest.terminal_id);
        let context = self.agent_registry.as_ref()
            .detection_context(
                manifest.terminal_id,
                manifest.cwd.clone(),
                &Default::default(),
            )
            .context(AGENT_CONTEXT_UNAVAILABLE)?;
        anyhow::ensure!(
            runtime.terminal_id == manifest.terminal_id
                && runtime.worker_pid != 0
                && runtime.worker_start_time != 0
                && !runtime.runtime_epoch.is_nil(),
            AGENT_CONTEXT_UNAVAILABLE
        );
        let authority = crate::agent::authority::AuthorityRuntimeIdentity {
            runtime_epoch: runtime.runtime_epoch,
            worker: crate::agent::authority::AuthorityProcessIdentity {
                pid: runtime.worker_pid,
                start_time: runtime.worker_start_time,
            },
        };
        store.load_published_authoritative_in_namespace(&self.namespace, &context, authority)
    }
}
