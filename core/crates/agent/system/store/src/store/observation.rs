use super::{AgentBinding, AgentBindingStore};
use anyhow::Result;

impl AgentBindingStore {
    pub fn read_raw_in_namespace(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
    ) -> Result<Option<AgentBinding>> {
        self.binding_file().read_in_namespace(namespace)
    }

    pub(super) fn read_raw_with_identity_in_namespace(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
    ) -> Result<Option<(AgentBinding, crate::paths::identity::FileIdentity)>> {
        self.binding_file()
            .read_with_identity_in_namespace(namespace)
    }

    pub(super) fn persist_reconciled_in_namespace(
        &self,
        namespace: &crate::namespace_lease::NamespaceLease,
        expected: Option<crate::paths::identity::FileIdentity>,
        binding: &AgentBinding,
        commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        self.binding_file().persist_reconciled_in_namespace(
            namespace,
            expected,
            binding,
            commit_fence,
        )
    }
}

pub(super) fn same(left: &AgentBinding, right: &AgentBinding) -> bool {
    left.schema_version == right.schema_version
        && left.terminal_id == right.terminal_id
        && left.kind == right.kind
        && left.provider_session_id == right.provider_session_id
        && left.cwd == right.cwd
        && left.evidence_path == right.evidence_path
        && left.data_home == right.data_home
        && left.process_pid == right.process_pid
        && left.process_start_time == right.process_start_time
}
