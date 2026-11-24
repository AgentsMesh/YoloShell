use super::*;

pub(crate) const NATIVE_RESUME_UNAUTHORIZED: &str =
    "native agent resume checkpoint is not authoritative";

impl Host {
    pub(crate) fn repair_native_binding_for_resume(
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
        store.repair_native_binding_in_namespace_with_commit_fence(
            &self.namespace,
            &context,
            runtime,
            || self.require_codex_resume_repair_open(manifest.terminal_id, runtime),
        )?;
        Ok(())
    }

    pub(crate) fn authorize_native_resume(
        &self,
        manifest: &TerminalManifest,
        binding: &AgentBinding,
    ) -> Result<()> {
        let runtime = self
            .agent_authority_runtime(manifest.terminal_id)?
            .context(NATIVE_RESUME_UNAUTHORIZED)?;
        let context = self.agent_registry.as_ref()
            .detection_context(
                manifest.terminal_id,
                manifest.cwd.clone(),
                &Default::default(),
            )
            .context(AGENT_CONTEXT_UNAVAILABLE)?;
        let store = self.agent_binding_store(manifest.terminal_id);
        anyhow::ensure!(
            store.authorize_native_resume_in_namespace(
                &self.namespace,
                &context,
                runtime,
                binding
            )?,
            NATIVE_RESUME_UNAUTHORIZED,
        );
        Ok(())
    }
}
