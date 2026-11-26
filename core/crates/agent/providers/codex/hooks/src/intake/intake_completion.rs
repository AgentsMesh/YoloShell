use super::{CodexHookIntake, CodexIntakeOccurrence};
use anyhow::Result;

impl CodexHookIntake {
    pub fn complete(&self, occurrence: &CodexIntakeOccurrence) -> Result<()> {
        self.complete_with_fence(occurrence, || Ok(()))
    }

    pub fn complete_with_fence<F>(&self, occurrence: &CodexIntakeOccurrence, fence: F) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        super::super::intake_storage::remove_exact_with_fence(
            &self.directory,
            &self.path(&occurrence.file_name),
            occurrence.identity,
            fence,
        )
    }
}
