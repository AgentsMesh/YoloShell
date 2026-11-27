use super::claude_recovery_authority::ClaudeBindingRecoveryAuthority;
use super::codex_recovery_authority::CodexBindingRecoveryAuthority;
use super::generic_recovery_authority::GenericBindingRecoveryAuthority;
use super::*;
use crate::agent::claude::generation::{
    AuthorityRuntimeIdentity, ClaudeGenerationAuthority, ClaudeGenerationStore,
};
use yolo_agent_hook_generation::HookGenerationStore;

impl AgentBindingStore {
    pub fn load_authoritative_in_namespace(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<Option<AgentBinding>> {
        self.load_with_runtime_authority_in_namespace(namespace, context, Some(runtime))
    }

    pub fn begin_managed_claude_in_namespace(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        begin: &crate::agent::claude::generation::GenerationBegin,
        active_is_foreground: bool,
    ) -> Result<ClaudeGenerationAuthority> {
        let generations = ClaudeGenerationStore::from_binding_path(&self.path)?;
        super::pending_recovery_orphans::reconcile(self, namespace, &generations)?;
        generations.begin_foreground_in_namespace(namespace, begin, active_is_foreground)
    }

    pub fn load_with_runtime_authority_in_namespace(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        context: &DetectionContext,
        runtime: Option<AuthorityRuntimeIdentity>,
    ) -> Result<Option<AgentBinding>> {
        if let Some(runtime) = runtime {
            if let Some(snapshot) =
                super::native_read::read(self, namespace, context, runtime, || Ok(()))?
            {
                return Ok(Some(snapshot.binding));
            }
        }
        let binding: Option<AgentBinding> = match namespace.read_json(&self.path) {
            Ok(binding) => Some(binding),
            Err(error) if is_missing(&error) => None,
            Err(error) if error.downcast_ref::<serde_json::Error>().is_some() => {
                return Err(error).context(super::super::constants::validation::DECODE);
            }
            Err(error) => {
                return Err(error).context(super::super::constants::validation::READ);
            }
        };
        let native_claude = super::native_authority::claude_record_exists(self, namespace)?;
        let generations = self.generation_store().filter(|_| !native_claude);
        if let Some(generations) = generations.as_ref() {
            super::pending_recovery_orphans::reconcile_isolated(self, namespace, generations)?;
        }
        let authority = generations
            .map(|store| store.load_in_namespace(namespace))
            .transpose()?
            .flatten();
        let codex_authority = if binding
            .as_ref()
            .is_some_and(|value| value.kind == crate::agent::AgentKind::Codex)
        {
            self.codex_authority_store()?.load_in_namespace(namespace)?
        } else {
            None
        };
        let generic_authority = HookGenerationStore::from_binding_path(&self.path)?
            .load_in_namespace(namespace)?;
        if let Some(historical) = super::historical_exit::load(
            self,
            namespace,
            context,
            runtime,
            authority.as_ref(),
            binding.as_ref(),
        )? {
            return Ok(Some(historical));
        }
        if let Some(recovered) = super::pending_recovery::recover(
            self,
            namespace,
            context,
            runtime,
            authority.as_ref(),
            binding.as_ref(),
        )? {
            return Ok(Some(recovered));
        }
        let Some(binding) = binding else {
            return Ok(None);
        };
        let claude_recovery_authority =
            match (binding.kind == crate::agent::AgentKind::ClaudeCode, runtime) {
                (true, Some(runtime)) => authority
                    .as_ref()
                    .map(|authority| ClaudeBindingRecoveryAuthority::new(authority, runtime)),
                _ => None,
            };
        let codex_recovery_authority =
            match (binding.kind == crate::agent::AgentKind::Codex, runtime) {
                (true, Some(runtime)) => codex_authority
                    .as_ref()
                    .map(|authority| CodexBindingRecoveryAuthority::new(authority, runtime)),
                _ => None,
            };
        let generic_recovery_authority = match (runtime, generic_authority.as_ref()) {
            (Some(runtime), Some(record)) if record.binding.kind == binding.kind => {
                Some(GenericBindingRecoveryAuthority::new(record, runtime))
            }
            _ => None,
        };
        let recovery_authority = codex_recovery_authority
            .as_ref()
            .map(|authority| authority as &dyn BindingRecoveryAuthority)
            .or_else(|| {
                claude_recovery_authority
                    .as_ref()
                    .map(|authority| authority as &dyn BindingRecoveryAuthority)
            })
            .or_else(|| {
                generic_recovery_authority
                    .as_ref()
                    .map(|authority| authority as &dyn BindingRecoveryAuthority)
            });
        let binding = recovery_safe(binding, &self.registry, recovery_authority);
        self.registry.validate_binding(&binding, context)?;
        Ok(Some(binding))
    }

    pub(crate) fn save_authoritative_in_namespace(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        binding: &AgentBinding,
        authority: &ClaudeGenerationAuthority,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<()> {
        anyhow::ensure!(
            authority.matches_runtime(runtime) && authority.matches_binding_payload(binding),
            super::super::constants::validation::GENERATION_AUTHORITY
        );
        namespace
            .write_json(&self.path, binding)
            .context(super::super::constants::validation::PERSIST)
    }

    pub fn invalidate_in_namespace(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        terminal_id: uuid::Uuid,
    ) -> Result<Option<AgentBinding>> {
        let mut binding = match self.read_raw_in_namespace(namespace)? {
            Some(binding) => binding,
            None => return Ok(None),
        };
        anyhow::ensure!(
            binding.terminal_id == terminal_id,
            super::super::constants::validation::YOLO_SESSION
        );
        if !matches!(binding.state, AgentState::Exited | AgentState::Unknown) {
            binding.state = AgentState::Unknown;
        }
        binding.recovery_availability = AgentRecoveryAvailability::Unavailable;
        binding.updated_at_ms = crate::model::now_ms();
        namespace
            .write_json(&self.path, &binding)
            .context(super::super::constants::validation::PERSIST)?;
        Ok(Some(binding))
    }

    pub(super) fn generation_store(&self) -> Option<ClaudeGenerationStore> {
        ClaudeGenerationStore::from_binding_path(&self.path).ok()
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/store/begin_availability_tests.rs"]
mod begin_availability_tests;
