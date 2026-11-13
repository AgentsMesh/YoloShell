use super::AgentBindingStore;
use crate::agent::claude::generation::{
    AuthorityRuntimeIdentity,
    native::ClaudeNativeGenerationStore,
};
use crate::agent::codex::authority::CodexAuthorityStatus;
use crate::namespace_lease::NamespaceLease;
use yolo_agent_hook_generation::HookGenerationStore;
use anyhow::Result;

impl AgentBindingStore {
    /// True when the terminal has a native authority that belongs to another
    /// runtime and none that matches this one. Such an authority cannot be
    /// adopted by a new Worker generation and must not be retried as if it
    /// were still settling.
    pub fn has_foreign_native_authority_in_namespace(
        &self,
        namespace: &NamespaceLease,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<bool> {
        let mut matching = false;
        let mut foreign = false;
        if let Some((record, _)) = ClaudeNativeGenerationStore::from_binding_path(self.path())?
            .load_with_identity_in_namespace(namespace)?
        {
            if !record.revoked {
                if record.matches_runtime(runtime) {
                    matching = true;
                } else {
                    foreign = true;
                }
            }
        }
        if let Some(record) = self.codex_authority_store()?.load_in_namespace(namespace)? {
            if record.status != CodexAuthorityStatus::Revoked && record.resume_boot_id().is_some() {
                if record.matches_runtime(runtime) {
                    matching = true;
                } else {
                    foreign = true;
                }
            }
        }
        if let Some(record) = HookGenerationStore::from_binding_path(self.path())?
            .load_in_namespace(namespace)?
        {
            if !record.revoked {
                if record.matches_runtime(runtime) {
                    matching = true;
                } else {
                    foreign = true;
                }
            }
        }
        Ok(foreign && !matching)
    }
}
