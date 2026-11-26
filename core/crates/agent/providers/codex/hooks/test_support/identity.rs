use crate::agent::hooks::{HookIdentity, ProviderVersion};
use anyhow::{Result, ensure};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CodexHookIdentity {
    pub(crate) hook: HookIdentity,
    pub(crate) provider_version: ProviderVersion,
    pub(crate) outbox_path: std::path::PathBuf,
}

impl CodexHookIdentity {
    pub(crate) fn for_test(
        paths: &crate::paths::Paths,
        hook: HookIdentity,
        provider_version: ProviderVersion,
    ) -> Result<Self> {
        hook.validate().map_err(anyhow::Error::new)?;
        ensure!(
            provider_version == super::ACCEPTED_PROVIDER_VERSION,
            super::constants::errors::VERSION_UNSUPPORTED
        );
        let artifacts = super::CodexHookArtifacts::new(
            paths,
            hook.terminal_id,
            hook.runtime_epoch,
            hook.invocation_id,
        )?;
        Ok(Self {
            hook,
            provider_version,
            outbox_path: artifacts.outbox_path(),
        })
    }
}
