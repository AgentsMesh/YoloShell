use super::recovery_authority::BindingRecoveryAuthority;
use crate::agent::AgentBinding;
use crate::agent::claude::generation::AuthorityRuntimeIdentity;
use crate::agent::codex::authority::CodexBindingAuthority;

pub(super) struct CodexBindingRecoveryAuthority<'a> {
    authority: &'a CodexBindingAuthority,
    runtime: AuthorityRuntimeIdentity,
}

impl<'a> CodexBindingRecoveryAuthority<'a> {
    pub(super) fn new(
        authority: &'a CodexBindingAuthority,
        runtime: AuthorityRuntimeIdentity,
    ) -> Self {
        Self { authority, runtime }
    }
}

impl BindingRecoveryAuthority for CodexBindingRecoveryAuthority<'_> {
    fn authorizes(&self, binding: &AgentBinding) -> bool {
        self.authority.authorizes(binding, self.runtime)
    }
}
