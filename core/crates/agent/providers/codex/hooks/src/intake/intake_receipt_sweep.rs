use super::{
    CodexAdmissionAuthority, CodexHookIntake, CodexOutbox, CodexRetirementSealArtifact,
    CodexRetirementWriterFence,
};
use anyhow::Result;

impl CodexHookIntake {
    pub fn recover_live_receipts(&self, outbox: &CodexOutbox) -> Result<usize> {
        let writer = self.acquire_writer_fence()?;
        outbox.sweep_occurrence_receipts(self, || {
            writer.verify_unsealed()?;
            self.invocation().verify_path()?;
            self.directory().verify_path()
        })
    }

    pub fn recover_sealed_receipts(
        &self,
        outbox: &CodexOutbox,
        writer: &CodexRetirementWriterFence,
        seal: &CodexRetirementSealArtifact,
        admission: &CodexAdmissionAuthority,
    ) -> Result<usize> {
        outbox.sweep_occurrence_receipts(self, || {
            writer.verify_seal(seal, admission)?;
            self.verify_admission(admission)?;
            writer.verify_seal(seal, admission)
        })
    }

    pub(super) fn complete_receipt_artifact(
        &self,
        identity: &super::CodexOccurrenceIdentity,
    ) -> Result<()> {
        super::intake_storage::remove_exact(
            self.directory(),
            &self.path(identity.file_name()),
            identity.file_identity(),
        )
    }
}
