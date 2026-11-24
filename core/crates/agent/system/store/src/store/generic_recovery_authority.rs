use super::recovery_authority::BindingRecoveryAuthority;
use crate::agent::AgentBinding;
use crate::agent::claude::generation::AuthorityRuntimeIdentity;
use yolo_agent_hook_generation::HookGenerationRecord;

pub(super) struct GenericBindingRecoveryAuthority<'a> {
    record: &'a HookGenerationRecord,
    runtime: AuthorityRuntimeIdentity,
}

impl<'a> GenericBindingRecoveryAuthority<'a> {
    pub(super) fn new(
        record: &'a HookGenerationRecord,
        runtime: AuthorityRuntimeIdentity,
    ) -> Self {
        Self { record, runtime }
    }
}

impl BindingRecoveryAuthority for GenericBindingRecoveryAuthority<'_> {
    fn authorizes(&self, binding: &AgentBinding) -> bool {
        self.record.matches_runtime(self.runtime) && self.record.authorizes_resume(binding)
    }
}
