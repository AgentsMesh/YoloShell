use super::{CodexHookArtifacts, CodexHookIntake};
use anyhow::{Context, Result};

impl CodexHookIntake {
    pub fn open_existing_for_identity(
        artifacts: &CodexHookArtifacts,
        owner: &crate::agent::hooks::HookIdentity,
    ) -> Result<Self> {
        let invocation = crate::paths::directory::PrivateDirectory::open(artifacts.directory())
            .context(super::constants::errors::INTAKE_STORAGE)?;
        Self::open_for_invocation(&invocation, owner)
    }
}
