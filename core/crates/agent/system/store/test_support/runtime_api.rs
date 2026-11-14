use crate::agent::claude::generation::{AuthorityRuntimeIdentity, ClaudeGenerationAuthority};
use crate::namespace_lease::NamespaceLease;
use crate::{AgentBinding, AgentBindingStore};
use anyhow::Result;

impl AgentBindingStore {
    pub fn save_authoritative_in_namespace_for_test(
        &self,
        namespace: &NamespaceLease,
        binding: &AgentBinding,
        authority: &ClaudeGenerationAuthority,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<()> {
        self.save_authoritative_in_namespace(namespace, binding, authority, runtime)
    }
}
