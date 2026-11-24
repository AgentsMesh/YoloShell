use super::{
    CodexAdmissionAuthority, CodexHookIntake, CodexInvocationAuthority,
    CodexRetirementSealArtifact, CodexRetirementWriterFence,
};
use crate::codex_hooks::retirement_constants::CORRUPT;
use anyhow::{Result, ensure};

pub struct CodexTerminalIntakeProof {
    intake: CodexHookIntake,
    invocation: CodexInvocationAuthority,
    admission: CodexAdmissionAuthority,
    seal: CodexRetirementSealArtifact,
}

impl CodexRetirementWriterFence {
    pub fn open_terminal_intake(
        &self,
        seal: &CodexRetirementSealArtifact,
        admission: &CodexAdmissionAuthority,
    ) -> Result<CodexHookIntake> {
        self.verify_seal(seal, admission)?;
        let expected = self.invocation_authority(admission)?;
        let intake = CodexHookIntake::open_for_invocation(
            self.storage.directory(),
            &admission.record().request().identity,
        )?;
        ensure!(
            CodexInvocationAuthority::capture(intake.invocation())? == expected,
            CORRUPT
        );
        self.verify_seal(seal, admission)?;
        Ok(intake)
    }

    pub fn prove_terminal_intake(
        &self,
        seal: &CodexRetirementSealArtifact,
        admission: &CodexAdmissionAuthority,
    ) -> Result<CodexTerminalIntakeProof> {
        let intake = self.open_terminal_intake(seal, admission)?;
        let proof = CodexTerminalIntakeProof {
            invocation: CodexInvocationAuthority::capture(intake.invocation())?,
            intake,
            admission: admission.clone(),
            seal: seal.clone(),
        };
        proof.verify(self, seal, admission)?;
        Ok(proof)
    }
}

impl CodexTerminalIntakeProof {
    pub(super) fn verify(
        &self,
        writer: &CodexRetirementWriterFence,
        seal: &CodexRetirementSealArtifact,
        admission: &CodexAdmissionAuthority,
    ) -> Result<()> {
        ensure!(self.admission.exactly_matches(admission), CORRUPT);
        ensure!(self.seal == *seal, CORRUPT);
        writer.verify_seal(seal, admission)?;
        ensure!(
            CodexInvocationAuthority::capture(self.intake.invocation())? == self.invocation,
            CORRUPT
        );
        ensure!(
            self.invocation == admission.invocation_authority()?,
            CORRUPT
        );
        let scan = self.intake.scan()?;
        ensure!(
            scan.occurrences.is_empty() && scan.deferred == usize::default(),
            CORRUPT
        );
        writer.verify_seal(seal, admission)
    }
}
