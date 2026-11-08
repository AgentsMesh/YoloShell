use super::super::authority_registry::CodexAuthorityRegistry;
use super::super::publication::CodexAuthorityView;
use super::{CodexHookRuntime, ReplayReport};
use crate::agent::ProcessSource;
use crate::agent::hooks::{HookIdentity, ProviderVersion};
use anyhow::{Context, Result, ensure};
use std::sync::Arc;
use uuid::Uuid;

type GenerationFence = super::super::cancellation::IngressCancellation;

pub(super) struct RuntimeBuildContext<'a> {
    pub(super) paths: &'a crate::paths::Paths,
    pub(super) registry: Arc<CodexAuthorityRegistry>,
    pub(super) terminal_id: Uuid,
    pub(super) runtime_epoch: Uuid,
    pub(super) shell: crate::process::ProcessIdentity,
    pub(super) owner: HookIdentity,
    pub(super) provider_version: ProviderVersion,
    pub(super) process_source: Arc<dyn ProcessSource>,
    pub(super) mode: RuntimeRecoveryMode<'a>,
    pub(super) generation: Option<GenerationFence>,
}

#[derive(Clone, Copy)]
pub(super) enum RuntimeRecoveryMode<'a> {
    Live,
    Sealed {
        writer: &'a crate::codex_hooks::CodexRetirementWriterFence,
        seal: &'a crate::codex_hooks::CodexRetirementSealArtifact,
        admission: &'a crate::codex_hooks::CodexAdmissionAuthority,
    },
}

impl RuntimeRecoveryMode<'_> {
    pub(super) fn verify(&self, owner: &HookIdentity) -> Result<()> {
        match self {
            Self::Live => Ok(()),
            Self::Sealed {
                writer,
                seal,
                admission,
            } => {
                ensure!(
                    &admission.record().request().identity == owner,
                    super::super::constants::CODEX_OWNER_ERROR
                );
                writer.verify_seal(seal, admission)
            }
        }
    }

    pub(super) fn requires_live_admission(&self) -> bool {
        matches!(self, Self::Live)
    }

    pub(super) fn initial_view(
        &self,
        lifecycle: &super::super::authority::LifecycleAuthority,
    ) -> Result<CodexAuthorityView> {
        match self {
            Self::Live => lifecycle
                .authority_view()
                .context(super::super::constants::CODEX_STORAGE_LATCHED_ERROR),
            Self::Sealed { .. } => Ok(CodexAuthorityView::StorageUnavailable),
        }
    }
}

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn with_process_source(
        paths: &crate::paths::Paths,
        registry: Arc<CodexAuthorityRegistry>,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        shell: crate::process::ProcessIdentity,
        owner: HookIdentity,
        provider_version: ProviderVersion,
        process_source: Arc<dyn ProcessSource>,
    ) -> Result<(Arc<Self>, ReplayReport)> {
        Self::with_process_source_generation(
            paths,
            registry,
            terminal_id,
            runtime_epoch,
            shell,
            owner,
            provider_version,
            process_source,
            None,
        )
    }

    pub(in crate::worker::codex_hook) fn recover_sealed_with_process_source<'a>(
        paths: &'a crate::paths::Paths,
        registry: Arc<CodexAuthorityRegistry>,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        shell: crate::process::ProcessIdentity,
        admission: &'a crate::codex_hooks::CodexAdmissionAuthority,
        writer: &'a crate::codex_hooks::CodexRetirementWriterFence,
        seal: &'a crate::codex_hooks::CodexRetirementSealArtifact,
        process_source: Arc<dyn ProcessSource>,
    ) -> Result<(Arc<Self>, ReplayReport)> {
        let request = admission.record().request();
        build_context(RuntimeBuildContext {
            paths,
            registry,
            terminal_id,
            runtime_epoch,
            shell,
            owner: request.identity.clone(),
            provider_version: request.provider_version,
            process_source,
            mode: RuntimeRecoveryMode::Sealed {
                writer,
                seal,
                admission,
            },
            generation: None,
        })
    }

    #[cfg(test)]
    pub(in crate::worker::codex_hook) fn new_test(
        paths: &crate::paths::Paths,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        shell: crate::process::ProcessIdentity,
        owner: HookIdentity,
        provider_version: ProviderVersion,
        process_source: Arc<dyn ProcessSource>,
    ) -> Result<(Arc<Self>, ReplayReport)> {
        Self::new_test_in_registry(
            paths,
            Arc::new(CodexAuthorityRegistry::default()),
            terminal_id,
            runtime_epoch,
            shell,
            owner,
            provider_version,
            process_source,
        )
    }

    #[cfg(test)]
    pub(in crate::worker::codex_hook) fn new_test_in_registry(
        paths: &crate::paths::Paths,
        registry: Arc<CodexAuthorityRegistry>,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        shell: crate::process::ProcessIdentity,
        owner: HookIdentity,
        provider_version: ProviderVersion,
        process_source: Arc<dyn ProcessSource>,
    ) -> Result<(Arc<Self>, ReplayReport)> {
        Self::with_process_source(
            paths,
            registry,
            terminal_id,
            runtime_epoch,
            shell,
            owner,
            provider_version,
            process_source,
        )
    }
}

include!("runtime_build_generation.rs");
