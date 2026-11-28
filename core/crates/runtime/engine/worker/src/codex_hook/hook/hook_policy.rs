use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const POLICY_INVALID: &str = "Codex hook admission policy is invalid";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CodexHookPolicy {
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    hook_executable: PathBuf,
}

impl CodexHookPolicy {
    pub(super) fn new(
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        hook_executable: PathBuf,
    ) -> Result<Self> {
        ensure!(
            !terminal_id.is_nil() && !runtime_epoch.is_nil(),
            POLICY_INVALID
        );
        crate::paths::validation::validate_absolute(&hook_executable)?;
        Ok(Self {
            terminal_id,
            runtime_epoch,
            hook_executable,
        })
    }

    pub(super) fn terminal_id(&self) -> Uuid {
        self.terminal_id
    }
    pub(super) fn runtime_epoch(&self) -> Uuid {
        self.runtime_epoch
    }
    pub(super) fn hook_executable(&self) -> &Path {
        &self.hook_executable
    }
}
