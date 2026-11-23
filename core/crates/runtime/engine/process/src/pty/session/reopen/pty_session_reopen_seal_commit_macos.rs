use super::{MacosPtyReopenSeal, SEAL_IDENTITY_DETAIL, SealPhase};
use crate::process::pty_session_proof::PtySessionProof;
use anyhow::{Result, ensure};

impl MacosPtyReopenSeal {
    pub(in crate::process::pty_session_proof) fn mark_destructive_fence_crossed(
        &mut self,
    ) -> Result<()> {
        if self.phase == SealPhase::Acquired {
            self.phase = SealPhase::FenceCrossed;
        }
        ensure!(
            matches!(self.phase, SealPhase::FenceCrossed | SealPhase::Committed),
            SEAL_IDENTITY_DETAIL
        );
        Ok(())
    }

    pub(in crate::process::pty_session_proof) fn commit(
        &mut self,
        proof: &PtySessionProof,
    ) -> Result<()> {
        #[cfg(test)]
        super::panic_hook::panic_if_requested(super::TestSealOperation::Commit);
        self.revalidate(proof)?;
        if matches!(self.phase, SealPhase::Acquired | SealPhase::FenceCrossed) {
            self.phase = SealPhase::Committed;
            drop(self.temporary_slave.take());
        }
        ensure!(self.phase == SealPhase::Committed, SEAL_IDENTITY_DETAIL);
        Ok(())
    }
}
