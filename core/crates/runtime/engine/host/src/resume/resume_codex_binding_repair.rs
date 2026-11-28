use super::*;

pub(crate) const CODEX_RESUME_REPAIR_FENCE_CHANGED: &str =
    "Codex resume binding repair authority changed";

impl Host {
    pub(crate) fn repair_codex_binding_for_resume(
        &self,
        manifest: &TerminalManifest,
    ) -> Result<()> {
        let Some(runtime) = self.agent_authority_runtime(manifest.terminal_id)? else {
            return Ok(());
        };
        self.require_codex_resume_repair_open(manifest.terminal_id, runtime)?;
        let store = self.agent_binding_store(manifest.terminal_id);
        let context = self.agent_registry.as_ref()
            .detection_context(
                manifest.terminal_id,
                manifest.cwd.clone(),
                &Default::default(),
            )
            .context(AGENT_CONTEXT_UNAVAILABLE)?;
        store
            .codex_authority_store()?
            .repair_active_binding_in_namespace_with_commit_fence(
                &self.namespace,
                store.path(),
                manifest.terminal_id,
                runtime,
                |binding| self.agent_registry.as_ref().validate_binding(binding, &context),
                || self.require_codex_resume_repair_open(manifest.terminal_id, runtime),
            )
    }

    pub(crate) fn require_codex_resume_repair_open(
        &self,
        terminal_id: Uuid,
        expected_runtime: crate::agent::authority::AuthorityRuntimeIdentity,
    ) -> Result<()> {
        self.namespace.verify()?;
        anyhow::ensure!(
            self.namespace
                .file_is_missing(&self.paths.resume_transaction(terminal_id))?
                && self
                    .namespace
                    .file_is_missing(&self.paths.resume_binding_clear(terminal_id))?
                && self.agent_authority_runtime(terminal_id)? == Some(expected_runtime),
            CODEX_RESUME_REPAIR_FENCE_CHANGED
        );
        self.namespace.verify()
    }
}
