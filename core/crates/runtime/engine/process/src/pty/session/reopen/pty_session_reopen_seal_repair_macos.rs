use super::{MacosPtyReopenSeal, SEALED_MODE, SealPhase};
use crate::process::pty_session_proof::{PtySessionProof, macos_reopen_seal_io as io};
use anyhow::{Context, Result};

const SEAL_REPAIR_FAILURE_CONTEXT: &str = "repair exact PTY reopen seal after committed failure";
const SEAL_REPAIR_INVALIDATED_DETAIL: &str =
    "exact PTY reopen-seal repair invalidated the observed output";

impl MacosPtyReopenSeal {
    pub(in crate::process::pty_session_proof) fn settle_after_fence(
        &mut self,
        proof: &PtySessionProof,
    ) -> Result<()> {
        if self.phase != SealPhase::Committed {
            return self.commit(proof);
        }
        let source = match self.revalidate(proof) {
            Ok(()) => return Ok(()),
            Err(source) => source,
        };
        self.ensure_physical_seal(proof, io::set_path_mode)
            .context(SEAL_REPAIR_FAILURE_CONTEXT)?;
        Err(source.context(SEAL_REPAIR_INVALIDATED_DETAIL))
    }

    pub(in crate::process) fn quarantine_fail_closed(
        &mut self,
        proof: &PtySessionProof,
    ) -> Result<()> {
        self.ensure_physical_seal(proof, io::set_path_mode_fail_closed)
            .context(SEAL_REPAIR_FAILURE_CONTEXT)?;
        #[cfg(test)]
        io::finalize_quarantine_for_test()?;
        self.phase = SealPhase::Committed;
        drop(self.temporary_slave.take());
        drop(self.master.take());
        Ok(())
    }

    fn ensure_physical_seal(
        &self,
        proof: &PtySessionProof,
        set_mode: fn(&std::path::Path, libc::mode_t) -> Result<()>,
    ) -> Result<()> {
        self.validate_authority(proof)?;
        match io::exact_path_seal_state(&self.path, self.object, SEALED_MODE)? {
            io::ExactPathSealState::Retired | io::ExactPathSealState::Sealed => Ok(()),
            io::ExactPathSealState::RepairRequired => {
                set_mode(&self.path, SEALED_MODE)?;
                io::validate_path_exact(&self.path, self.object, SEALED_MODE)
            }
        }
    }
}
