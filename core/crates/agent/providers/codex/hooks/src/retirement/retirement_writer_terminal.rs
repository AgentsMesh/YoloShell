use super::CodexRetirementWriterFence;
use crate::agent::hooks::ReducerSnapshot;
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexRetirementArtifact, CodexRetirementPublication,
    CodexRetirementRecord, CodexRetirementSealArtifact, CodexSnapshotProof,
    CodexTerminalIntakeProof, CodexTerminalOutboxProof,
};
use anyhow::Result;

impl CodexRetirementWriterFence {
    pub fn publish_retirement(
        &self,
        record: &CodexRetirementRecord,
        seal: &CodexRetirementSealArtifact,
        snapshot: &ReducerSnapshot,
        admission: &CodexAdmissionAuthority,
        proof: &CodexSnapshotProof,
        intake: &CodexTerminalIntakeProof,
        outbox: &CodexTerminalOutboxProof,
    ) -> Result<CodexRetirementPublication> {
        self.verify_terminal_drained(record, seal, snapshot, admission, proof, intake, outbox)?;
        let publication = self.storage.publish_retirement(record, || {
            self.verify_terminal_drained(record, seal, snapshot, admission, proof, intake, outbox)
        })?;
        self.verify_retirement_drained(
            publication.artifact(),
            seal,
            snapshot,
            admission,
            proof,
            intake,
            outbox,
        )?;
        Ok(publication)
    }

    pub(crate) fn verify_retirement(
        &self,
        artifact: &CodexRetirementArtifact,
        seal: &CodexRetirementSealArtifact,
        snapshot: &ReducerSnapshot,
        admission: &CodexAdmissionAuthority,
        proof: &CodexSnapshotProof,
    ) -> Result<()> {
        self.verify_terminal_record(artifact.record(), seal, snapshot, admission, proof)?;
        self.storage.verify_retirement(artifact)?;
        self.verify_terminal_record(artifact.record(), seal, snapshot, admission, proof)
    }

    pub fn verify_retirement_drained(
        &self,
        artifact: &CodexRetirementArtifact,
        seal: &CodexRetirementSealArtifact,
        snapshot: &ReducerSnapshot,
        admission: &CodexAdmissionAuthority,
        proof: &CodexSnapshotProof,
        intake: &CodexTerminalIntakeProof,
        outbox: &CodexTerminalOutboxProof,
    ) -> Result<()> {
        self.verify_terminal_drained(
            artifact.record(),
            seal,
            snapshot,
            admission,
            proof,
            intake,
            outbox,
        )?;
        self.storage.verify_retirement(artifact)?;
        self.verify_terminal_drained(
            artifact.record(),
            seal,
            snapshot,
            admission,
            proof,
            intake,
            outbox,
        )
    }

    fn verify_terminal_drained(
        &self,
        record: &CodexRetirementRecord,
        seal: &CodexRetirementSealArtifact,
        snapshot: &ReducerSnapshot,
        admission: &CodexAdmissionAuthority,
        proof: &CodexSnapshotProof,
        intake: &CodexTerminalIntakeProof,
        outbox: &CodexTerminalOutboxProof,
    ) -> Result<()> {
        self.verify_terminal_record(record, seal, snapshot, admission, proof)?;
        intake.verify(self, seal, admission)?;
        outbox.verify(snapshot, admission)?;
        self.verify_terminal_record(record, seal, snapshot, admission, proof)
    }

    fn verify_terminal_record(
        &self,
        record: &CodexRetirementRecord,
        seal: &CodexRetirementSealArtifact,
        snapshot: &ReducerSnapshot,
        admission: &CodexAdmissionAuthority,
        proof: &CodexSnapshotProof,
    ) -> Result<()> {
        let authority = self.invocation_authority(admission)?;
        self.storage.verify_seal(seal)?;
        record.validate_for(snapshot, admission, seal, proof)?;
        record.require_invocation(authority)?;
        self.storage.verify_seal(seal)?;
        self.verify_invocation_authority(authority, admission)
    }
}
