use super::ClaudeGenerationStore;
use super::constants::*;
#[cfg(test)]
use crate::paths::private_file::{PrivateDirectoryLock, PrivateFileEntry};
use anyhow::{Context, Result};
use std::path::PathBuf;

impl ClaudeGenerationStore {
    pub(super) fn binding_path(&self) -> Result<PathBuf> {
        Ok(self
            .path
            .parent()
            .context(BINDING_PARENT_MISSING)?
            .join(crate::agent::constants::BINDING_FILE_NAME))
    }

    #[cfg(test)]
    pub(super) fn lock_binding_lifecycle(&self) -> Result<PrivateDirectoryLock> {
        PrivateFileEntry::ensure(&self.binding_path()?)?
            .lock_parent_exclusive_wait(DURABLE_STATE_LOCK_TIMEOUT, DURABLE_STATE_LOCK_RETRY)
            .context(DURABLE_STATE_LOCK_CONTEXT)
    }
}
