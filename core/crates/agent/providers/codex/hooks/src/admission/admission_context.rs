use crate::agent::hooks::{Generation, HookIdentity, ProviderVersion};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CONTEXT_INVALID: &str = "Codex admission context is invalid";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexAdmissionContext {
    pub identity: HookIdentity,
    pub provider_version: ProviderVersion,
    pub cwd: PathBuf,
    pub data_home: PathBuf,
    pub hook_executable: PathBuf,
    pub real_executable: PathBuf,
}

impl CodexAdmissionContext {
    pub fn new(
        identity: HookIdentity,
        cwd: PathBuf,
        data_home: PathBuf,
        hook_executable: PathBuf,
        real_executable: PathBuf,
    ) -> Self {
        Self {
            identity,
            provider_version: super::ACCEPTED_PROVIDER_VERSION,
            cwd,
            data_home,
            hook_executable,
            real_executable,
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.identity.validate().map_err(anyhow::Error::new)?;
        ensure!(
            self.identity.generation == Generation::INITIAL
                && self.provider_version == super::ACCEPTED_PROVIDER_VERSION,
            CONTEXT_INVALID
        );
        for path in [
            &self.cwd,
            &self.data_home,
            &self.hook_executable,
            &self.real_executable,
        ] {
            crate::paths::validation::validate_absolute(path)?;
        }
        Ok(())
    }
}
