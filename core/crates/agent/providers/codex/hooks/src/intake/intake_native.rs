use super::CodexHookIntake;
use crate::agent::hooks::HookEnvelope;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Result, ensure};

const NATIVE_PUBLICATION_LOCK: &str = ".native-publication";

impl CodexHookIntake {
    pub fn persist_explicit_occurrence(&self, event: HookEnvelope) -> Result<HookEnvelope> {
        ensure!(
            event.identity == *self.owner(),
            super::super::constants::errors::INTAKE_CORRUPT
        );
        let entry = PrivateFileEntry::from_directory(
            self.directory().try_clone()?,
            &self.path(NATIVE_PUBLICATION_LOCK),
        )?;
        let _lock = entry.lock_parent_exclusive_wait(
            super::super::constants::LOCK_TIMEOUT,
            super::super::constants::LOCK_RETRY,
        )?;
        let fence = self.acquire_writer_fence()?;
        fence.verify_unsealed()?;
        if let Some(existing) = self.find_exact_occurrence(&event)? {
            return Ok(existing.envelope().clone());
        }
        let record = super::super::intake_record::CodexIntakeRecord::capture_explicit(event)?;
        self.verify_occurrence_capacity(false)?;
        super::super::intake_storage::create_with_fence(
            self.directory(),
            &self.path(&record.pending_file_name()),
            &record,
            || {
                fence.verify_unsealed()?;
                self.verify_occurrence_capacity(true)
            },
        )?;
        Ok(record.envelope().clone())
    }
}
