use super::*;
use crate::agent::claude::generation::{AuthorityRuntimeIdentity, ClaudeGenerationStore};
use yolo_agent_hook_generation::HookGenerationStore;

impl AgentBindingStore {
    pub fn load_published_authoritative_in_namespace(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<Option<AgentBinding>> {
        self.load_published_authoritative_with_snapshot_hook(namespace, context, runtime, || Ok(()))
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    #[doc(hidden)]
    pub fn load_published_authoritative_with_test_hook(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
        after_snapshot: impl FnOnce() -> Result<()>,
    ) -> Result<Option<AgentBinding>> {
        self.load_published_authoritative_with_snapshot_hook(
            namespace,
            context,
            runtime,
            after_snapshot,
        )
    }

    fn load_published_authoritative_with_snapshot_hook(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
        after_snapshot: impl FnOnce() -> Result<()>,
    ) -> Result<Option<AgentBinding>> {
        let mut after_snapshot = Some(after_snapshot);
        if let Some(snapshot) = super::native_read::read(self, namespace, context, runtime, || {
            after_snapshot
                .take()
                .context(super::native_authority::INVALID)?()
        })? {
            return Ok((snapshot.raw.as_ref().map(|(binding, _)| binding)
                == Some(&snapshot.binding))
            .then_some(snapshot.binding));
        }
        let after_snapshot = after_snapshot
            .take()
            .context(super::native_authority::INVALID)?;
        let Some(binding) = self.read_raw_in_namespace(namespace)? else {
            return Ok(None);
        };
        if binding.kind == crate::agent::AgentKind::Codex {
            return self.load_published_codex_authority(
                namespace,
                context,
                runtime,
                binding,
                after_snapshot,
            );
        }
        if binding.kind != crate::agent::AgentKind::ClaudeCode {
            if HookGenerationStore::from_binding_path(self.path())?
                .load_in_namespace(namespace)?
                .is_some()
            {
                return Ok(None);
            }
            let binding = recovery_safe(binding, &self.registry, None);
            self.registry.validate_binding(&binding, context)?;
            return Ok(Some(binding));
        }
        if super::native_authority::claude_record_exists(self, namespace)? {
            return Ok(None);
        }
        let generations = ClaudeGenerationStore::from_binding_path(self.path())?;
        let Some(authority) = generations.load_in_namespace(namespace)? else {
            return Ok(None);
        };
        after_snapshot()?;
        if !authority.matches_runtime(runtime) || !authority.matches_binding(&binding) {
            return Ok(None);
        }
        let published = match generations.binding_is_published_in_namespace(
            namespace,
            authority.authentication(),
            &authority,
        ) {
            Ok(published) => published,
            Err(error) => {
                if generations.load_in_namespace(namespace)? != Some(authority) {
                    return Ok(None);
                }
                return Err(error);
            }
        };
        if !published
            || generations.load_in_namespace(namespace)? != Some(authority)
            || self.read_raw_in_namespace(namespace)?.as_ref() != Some(&binding)
        {
            return Ok(None);
        }
        self.registry.validate_binding(&binding, context)?;
        Ok(Some(binding))
    }

    fn load_published_codex_authority(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        context: &DetectionContext,
        runtime: AuthorityRuntimeIdentity,
        binding: AgentBinding,
        after_snapshot: impl FnOnce() -> Result<()>,
    ) -> Result<Option<AgentBinding>> {
        let store = self.codex_authority_store()?;
        let Some(authority) = store.load_in_namespace(namespace)? else {
            return Ok(None);
        };
        after_snapshot()?;
        if !authority.authorizes(&binding, runtime)
            || store.load_in_namespace(namespace)?.as_ref() != Some(&authority)
            || self.read_raw_in_namespace(namespace)?.as_ref() != Some(&binding)
        {
            return Ok(None);
        }
        self.registry.validate_binding(&binding, context)?;
        Ok(Some(binding))
    }
}
