use super::CodexRetirementWriterFence;
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexRetirementArtifact, CodexRetirementSealArtifact,
    CodexSnapshotCursor,
};
use anyhow::{Context, Result};

pub enum CodexRetirementState {
    Unsealed,
    Sealed(CodexRetirementSealArtifact),
    Retired(CodexRetiredState),
}

pub struct CodexRetiredState {
    seal: CodexRetirementSealArtifact,
    retirement: CodexRetirementArtifact,
}

impl CodexRetirementWriterFence {
    pub fn read_state(&self, admission: &CodexAdmissionAuthority) -> Result<CodexRetirementState> {
        let invocation = self.invocation_authority(admission)?;
        let seal = self.storage.read_seal()?;
        let retirement = self.storage.read_retirement()?;
        let state = match (seal, retirement) {
            (None, None) => {
                self.storage.ensure_unsealed()?;
                CodexRetirementState::Unsealed
            }
            (Some(seal), None) => {
                self.verify_seal(&seal, admission)?;
                self.storage.verify_seal_only(&seal)?;
                CodexRetirementState::Sealed(seal)
            }
            (Some(seal), Some(retirement)) => {
                let retired = self.read_retired(seal, retirement, admission)?;
                CodexRetirementState::Retired(retired)
            }
            (None, Some(_)) => return Err(corrupt()),
        };
        self.verify_invocation_authority(invocation, admission)?;
        Ok(state)
    }

    fn read_retired(
        &self,
        seal: CodexRetirementSealArtifact,
        retirement: CodexRetirementArtifact,
        admission: &CodexAdmissionAuthority,
    ) -> Result<CodexRetiredState> {
        let (cursor, snapshot) =
            CodexSnapshotCursor::open_in(self.storage.directory(), self.storage.snapshot_path())?;
        let snapshot = snapshot.context(crate::codex_hooks::retirement_constants::CORRUPT)?;
        let proof = cursor.prove_current()?;
        self.verify_retirement(&retirement, &seal, &snapshot, admission, &proof)?;
        Ok(CodexRetiredState { seal, retirement })
    }
}

impl CodexRetiredState {
    pub fn seal(&self) -> &CodexRetirementSealArtifact {
        &self.seal
    }

    pub fn retirement(&self) -> &CodexRetirementArtifact {
        &self.retirement
    }
}

fn corrupt() -> anyhow::Error {
    anyhow::anyhow!(crate::codex_hooks::retirement_constants::CORRUPT)
}
