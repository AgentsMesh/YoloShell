impl RuntimeBuildContext<'_> {
    pub(super) fn verify_generation(&self) -> Result<()> {
        if self
            .generation
            .as_ref()
            .is_some_and(GenerationFence::is_cancelled)
        {
            anyhow::bail!(super::super::constants::CODEX_INGRESS_GENERATION_REVOKED_ERROR);
        }
        Ok(())
    }

    pub(super) fn acquire_generation_fence(
        &self,
    ) -> Result<Option<super::super::cancellation::IngressGenerationFence>> {
        self.generation
            .as_ref()
            .map(|generation| {
                generation.try_fence().ok_or_else(|| {
                    anyhow::anyhow!(super::super::constants::CODEX_INGRESS_GENERATION_REVOKED_ERROR)
                })
            })
            .transpose()
    }
}

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn with_process_source_generation(
        paths: &crate::paths::Paths,
        registry: Arc<CodexAuthorityRegistry>,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        shell: crate::process::ProcessIdentity,
        owner: HookIdentity,
        provider_version: ProviderVersion,
        process_source: Arc<dyn ProcessSource>,
        generation: Option<GenerationFence>,
    ) -> Result<(Arc<Self>, ReplayReport)> {
        build_context(RuntimeBuildContext {
            paths,
            registry,
            terminal_id,
            runtime_epoch,
            shell,
            owner,
            provider_version,
            process_source,
            mode: RuntimeRecoveryMode::Live,
            generation,
        })
    }
}

fn build_context(
    context: RuntimeBuildContext<'_>,
) -> Result<(Arc<CodexHookRuntime>, ReplayReport)> {
    super::build_support::validate_binding(
        context.terminal_id,
        context.runtime_epoch,
        context.shell,
        &context.owner,
        context.provider_version,
    )?;
    context.mode.verify(&context.owner)?;
    super::takeover::build(context)
}
