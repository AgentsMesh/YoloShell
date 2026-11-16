use super::intake_record::CodexIntakeRecord;
use super::{CodexAdmissionAuthority, CodexHookIntake, CodexInvocationAuthority};
use anyhow::{Result, ensure};

#[path = "intake_native.rs"]
mod native;

impl CodexHookIntake {
    pub(crate) fn persist(&self, envelope: crate::agent::hooks::HookEnvelope) -> Result<()> {
        self.persist_occurrence(envelope).map(drop)
    }

    pub(crate) fn persist_occurrence(
        &self,
        envelope: crate::agent::hooks::HookEnvelope,
    ) -> Result<crate::agent::hooks::HookEnvelope> {
        ensure!(
            envelope.identity == *self.owner(),
            super::constants::errors::INTAKE_CORRUPT
        );
        let record = CodexIntakeRecord::capture(envelope)?;
        let path = self.path(&record.pending_file_name());
        let fence = self.acquire_writer_fence()?;
        fence.verify_unsealed()?;
        self.verify_occurrence_capacity(false)?;
        super::intake_storage::create_with_fence(self.directory(), &path, &record, || {
            fence.verify_unsealed()?;
            self.verify_occurrence_capacity(true)
        })?;
        Ok(record.envelope().clone())
    }

    pub fn verify_admission(&self, admission: &CodexAdmissionAuthority) -> Result<()> {
        admission.verify()?;
        ensure!(
            admission.record().request().identity == *self.owner(),
            super::constants::errors::INTAKE_CORRUPT
        );
        ensure!(
            admission.invocation_authority()?
                == CodexInvocationAuthority::capture(self.invocation())?,
            super::constants::errors::INTAKE_CORRUPT
        );
        admission.verify()
    }
}
