use super::recovery_authority::BindingRecoveryAuthority;
use crate::agent::AgentBinding;
use crate::agent::claude::generation::{AuthorityRuntimeIdentity, ClaudeGenerationAuthority};

pub(super) struct ClaudeBindingRecoveryAuthority<'a> {
    authority: &'a ClaudeGenerationAuthority,
    runtime: AuthorityRuntimeIdentity,
}

impl<'a> ClaudeBindingRecoveryAuthority<'a> {
    pub(super) fn new(
        authority: &'a ClaudeGenerationAuthority,
        runtime: AuthorityRuntimeIdentity,
    ) -> Self {
        Self { authority, runtime }
    }
}

impl BindingRecoveryAuthority for ClaudeBindingRecoveryAuthority<'_> {
    fn authorizes(&self, binding: &AgentBinding) -> bool {
        self.authority.matches_runtime(self.runtime) && self.authority.matches_binding(binding)
    }
}
