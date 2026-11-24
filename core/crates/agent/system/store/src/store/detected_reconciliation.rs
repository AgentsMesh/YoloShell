use super::claude_recovery_authority::ClaudeBindingRecoveryAuthority;
use super::recovery_authority::BindingRecoveryAuthority;
use super::{AgentBindingStore, recovery_safe};
use crate::DetectionContext;
use crate::agent::claude::generation::{AuthorityRuntimeIdentity, ClaudeGenerationAuthority};
use crate::agent::{AgentBinding, AgentRecoveryAvailability, AgentState};
use anyhow::Result;

impl AgentBindingStore {
    pub(crate) fn reconcile_detected_in_namespace(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        binding: &AgentBinding,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<AgentBinding> {
        self.reconcile_detected_with_commit_fence(namespace, binding, runtime, || Ok(()))
    }

    pub fn reconcile_detected_with_commit_fence(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        binding: &AgentBinding,
        runtime: AuthorityRuntimeIdentity,
        commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<AgentBinding> {
        if let Some(snapshot) = super::native_read::read(
            self,
            namespace,
            &DetectionContext::new(
                binding.terminal_id,
                std::path::PathBuf::from(&binding.cwd),
                [(
                    binding.kind.clone(),
                    std::path::PathBuf::from(&binding.data_home),
                )],
            ),
            runtime,
            || Ok(()),
        )? {
            if snapshot.authority.matches_observation(binding) {
                let mut commit_fence = commit_fence;
                if snapshot.raw.as_ref().map(|(binding, _)| binding) == Some(&snapshot.binding) {
                    commit_fence()?;
                    snapshot.verify(self, namespace, runtime)?;
                    return Ok(snapshot.binding);
                }
                self.persist_reconciled_in_namespace(
                    namespace,
                    snapshot.raw.as_ref().map(|(_, identity)| *identity),
                    &snapshot.binding,
                    || {
                        commit_fence()?;
                        snapshot.verify_source(self, namespace, runtime)
                    },
                )?;
                return Ok(snapshot.binding);
            }
        }
        let loaded = self.read_raw_with_identity_in_namespace(namespace)?;
        let expected = loaded.as_ref().map(|(_, identity)| *identity);
        let current = loaded.map(|(binding, _)| binding);
        let codex = binding.kind == crate::agent::AgentKind::Codex
            || current
                .as_ref()
                .is_some_and(|value| value.kind == crate::agent::AgentKind::Codex);
        let codex_authority = if codex {
            self.codex_authority_store()?.load_in_namespace(namespace)?
        } else {
            None
        };
        if let (Some(authority), Some(current)) = (codex_authority.as_ref(), current.as_ref()) {
            if authority.authorizes(current, runtime) {
                self.persist_reconciled_in_namespace(namespace, expected, current, commit_fence)?;
                return Ok(current.clone());
            }
        }
        let claude = binding.kind == crate::agent::AgentKind::ClaudeCode
            || current
                .as_ref()
                .is_some_and(|value| value.kind == crate::agent::AgentKind::ClaudeCode);
        let authority =
            if claude && !super::native_authority::claude_record_exists(self, namespace)? {
                self.generation_store()
                    .map(|store| store.load_in_namespace(namespace))
                    .transpose()?
                    .flatten()
            } else {
                None
            };
        let canonical = || {
            super::hook_reconciliation::canonical(
                authority.as_ref()?,
                current.as_ref()?,
                binding,
                runtime,
            )
        };
        let retained = || {
            super::hook_reconciliation::retain_rejected_unmanaged(
                authority.as_ref()?,
                current.as_ref()?,
                binding,
                runtime,
            )
        };
        let state =
            || reconcile_hook_state(authority.as_ref()?, current.as_ref()?, binding, runtime);
        let recovery_authority = authority
            .as_ref()
            .map(|value| ClaudeBindingRecoveryAuthority::new(value, runtime));
        let persisted = canonical()
            .or_else(retained)
            .or_else(state)
            .unwrap_or_else(|| {
                recovery_safe(
                    binding.clone(),
                    self.registry(),
                    recovery_authority
                        .as_ref()
                        .map(|value| value as &dyn BindingRecoveryAuthority),
                )
            });
        self.persist_reconciled_in_namespace(namespace, expected, &persisted, commit_fence)?;
        Ok(persisted)
    }
}

fn reconcile_hook_state(
    authority: &ClaudeGenerationAuthority,
    current: &AgentBinding,
    binding: &AgentBinding,
    runtime: AuthorityRuntimeIdentity,
) -> Option<AgentBinding> {
    if binding.recovery_availability == AgentRecoveryAvailability::UnmanagedLaunch
        || !authority.matches_runtime(runtime)
        || !authority.matches_binding(current)
        || !super::observation::same(binding, current)
    {
        return None;
    }
    let mut reconciled = current.clone();
    if binding.state != current.state {
        reconciled.state = AgentState::Unknown;
        reconciled.updated_at_ms = crate::model::now_ms();
    }
    Some(reconciled)
}
