use super::*;

pub(crate) const RESUME_PROVIDER_BINDING_MISSING: &str =
    "provider resume has not published an authoritative agent binding";
pub(crate) const RESUME_PROVIDER_BINDING_MISMATCH: &str =
    "provider resume published an unexpected agent binding";
pub(crate) const RESUME_PROVIDER_STARTUP_RECEIPT_MISSING: &str =
    "provider resume has not published its durable startup receipt";

impl Host {
    pub(crate) async fn ensure_provider_completion(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> ProviderCompletionResult<ResumeTransaction> {
        match (&transaction.target, &transaction.provider_completion) {
            (ResumeTarget::Shell { .. }, None) => Ok(transaction.clone()),
            (ResumeTarget::Provider { .. }, Some(authority)) => {
                authority
                    .validate_for(transaction)
                    .map_err(ProviderCompletionError::contradiction)?;
                authority
                    .validate_runtime(runtime)
                    .map_err(ProviderCompletionError::contradiction)?;
                Ok(transaction.clone())
            }
            (ResumeTarget::Provider { .. }, None) => self
                .await_authoritative_resume_binding(manifest, transaction, runtime)
                .await?
                .context(RESUME_PROVIDER_AUTHORITY_INVALID)
                .map_err(ProviderCompletionError::contradiction),
            (ResumeTarget::Shell { .. }, Some(_)) => Err(ProviderCompletionError::contradiction(
                anyhow::anyhow!(RESUME_PROVIDER_AUTHORITY_INVALID),
            )),
        }
    }

    pub(crate) fn completed_provider_binding(
        &self,
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
    ) -> Result<Option<AgentBinding>> {
        match (&transaction.target, &transaction.provider_completion) {
            (ResumeTarget::Shell { .. }, None) => Ok(None),
            (ResumeTarget::Provider { .. }, Some(authority)) => {
                authority.validate_for(transaction)?;
                authority.validate_runtime(runtime)?;
                Ok(Some(authority.binding.clone()))
            }
            (ResumeTarget::Provider { .. }, None) => {
                anyhow::bail!(RESUME_PROVIDER_AUTHORITY_INVALID)
            }
            (ResumeTarget::Shell { .. }, Some(_)) => {
                anyhow::bail!(RESUME_PROVIDER_AUTHORITY_INVALID)
            }
        }
    }

    pub(crate) fn provider_startup_receipt_ready(
        &self,
        runtime: &RuntimeRecord,
        binding: &AgentBinding,
    ) -> Result<bool> {
        let provider = self
            .config
            .provider_runtime
            .get(&binding.kind)
            .context(RESUME_PROVIDER_STARTUP_RECEIPT_MISSING)?;
        if provider.adapter().session_generation_policy()
            != crate::agent::SessionGenerationPolicy::HookRequired
        {
            return Ok(true);
        }
        match provider.descriptor().authority_kind() {
            yolo_agent_providers::AuthorityKind::CodexOccurrence => {
                return self.codex_startup_receipt_ready(runtime, binding);
            }
            yolo_agent_providers::AuthorityKind::ClaudeGeneration => {}
            yolo_agent_providers::AuthorityKind::GenericHookGeneration => {
                return self.generic_startup_receipt_ready(runtime, binding);
            }
            yolo_agent_providers::AuthorityKind::None => return Ok(false),
        }
        let store = crate::agent::claude::generation::native::ClaudeNativeGenerationStore::from_binding_path(
            &self.paths.agent_binding(runtime.terminal_id),
        )?;
        let Some(record) = store.load_in_namespace(&self.namespace)? else {
            return Ok(false);
        };
        let authority = crate::agent::authority::AuthorityRuntimeIdentity {
            runtime_epoch: runtime.runtime_epoch,
            worker: crate::agent::authority::AuthorityProcessIdentity {
                pid: runtime.worker_pid,
                start_time: runtime.worker_start_time,
            },
        };
        Ok(record.matches_runtime(authority)
            && record.boot_id == runtime.boot_id
            && record.authorizes_resume(binding)
            && record.recovery_binding(true) == *binding
            && record.process().matches())
    }

    pub(crate) fn generic_startup_receipt_ready(
        &self,
        runtime: &RuntimeRecord,
        binding: &AgentBinding,
    ) -> Result<bool> {
        if binding.terminal_id != runtime.terminal_id {
            return Ok(false);
        }
        let authority_runtime = crate::agent::authority::AuthorityRuntimeIdentity {
            runtime_epoch: runtime.runtime_epoch,
            worker: crate::agent::authority::AuthorityProcessIdentity {
                pid: runtime.worker_pid,
                start_time: runtime.worker_start_time,
            },
        };
        let store = yolo_agent_hook_generation::HookGenerationStore::from_binding_path(
            &self.paths.agent_binding(runtime.terminal_id),
        )?;
        let Some(record) = store.load_in_namespace(&self.namespace)? else {
            return Ok(false);
        };
        Ok(record.matches_runtime(authority_runtime)
            && record.boot_id == runtime.boot_id
            && record.authorizes_resume(binding)
            && record.recovery_binding(true) == *binding
            && crate::process::process_identity_relation(
                record.scope.agent.pid,
                record.scope.agent.start_time,
            ) == crate::process::ProcessIdentityRelation::Matching)
    }

    pub(crate) fn codex_startup_receipt_ready(
        &self,
        runtime: &RuntimeRecord,
        binding: &AgentBinding,
    ) -> Result<bool> {
        if binding.terminal_id != runtime.terminal_id {
            return Ok(false);
        }
        let authority_runtime = crate::agent::authority::AuthorityRuntimeIdentity {
            runtime_epoch: runtime.runtime_epoch,
            worker: crate::agent::authority::AuthorityProcessIdentity {
                pid: runtime.worker_pid,
                start_time: runtime.worker_start_time,
            },
        };
        let store = self.agent_binding_store(runtime.terminal_id);
        let Some(authority) = store
            .codex_authority_store()?
            .load_in_namespace(&self.namespace)?
        else {
            return Ok(false);
        };
        Ok(authority.resume_boot_id() == Some(runtime.boot_id.as_str())
            && authority.verify_resume_evidence().is_ok()
            && authority.authorizes(binding, authority_runtime))
    }

    pub(crate) async fn completed_resume_view(
        &self,
        manifest: TerminalManifest,
        provider: Option<AgentBinding>,
    ) -> TerminalView {
        let Some(agent) = provider else {
            return self.view_for_manifest(manifest).await;
        };
        let runtime = self.runtime_view_for_manifest(&manifest).await;
        TerminalView {
            manifest,
            runtime,
            agent: Some(agent),
            completed_attachment: None,
        }
    }
}
