use super::AgentBindingStore;
use super::native_authority::INVALID;
use crate::agent::claude::generation::native::ClaudeNativeGenerationStore;
use crate::agent::codex::authority::CodexBindingAuthority;
use yolo_agent_hook_generation::HookGenerationStore;
use crate::paths::durable_mutation::SessionDurableMutationGuard;
use anyhow::{Result, ensure};

impl AgentBindingStore {
    pub fn revoke_native_authorities_with_guard(
        &self,
        guard: &SessionDurableMutationGuard<'_>,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        let namespace = guard.namespace();
        guard.require_managed_path(self.path())?;
        let claude = ClaudeNativeGenerationStore::from_binding_path(self.path())?;
        let claude_snapshot = claude.load_with_identity_in_namespace(namespace)?;
        let codex = self.codex_authority_store()?;
        let codex_snapshot: Option<(CodexBindingAuthority, _)> =
            if namespace.file_is_missing(codex.path())? {
                None
            } else {
                Some(namespace.read_json_with_identity(codex.path())?)
            };
        if let Some((record, _)) = &claude_snapshot {
            ensure!(record.scope.terminal_id == guard.terminal_id(), INVALID);
        }
        if let Some((record, _)) = &codex_snapshot {
            record.validate()?;
            ensure!(
                record.binding.terminal_id == guard.terminal_id(),
                INVALID
            );
        }
        if let Some((record, identity)) = claude_snapshot {
            claude.revoke_all_with_guard(guard, &record, identity, &mut commit_fence)?;
        }
        if let Some((record, identity)) = codex_snapshot {
            codex.revoke_exact_in_namespace_with_commit_fence(
                namespace,
                &record,
                identity,
                &mut commit_fence,
            )?;
        }
        HookGenerationStore::from_binding_path(self.path())?.revoke_in_namespace(
            namespace,
            guard.terminal_id(),
            &mut commit_fence,
        )?;
        commit_fence()
    }
}
