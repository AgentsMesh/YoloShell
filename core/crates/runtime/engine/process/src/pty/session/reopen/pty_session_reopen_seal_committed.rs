use super::{
    CommittedPtyReopenSealAuthority, NotApplicableReason, REOPEN_SEAL_AUTHORITY_MISMATCH_DETAIL,
    ReopenSealAuthority,
};
#[cfg(not(target_os = "macos"))]
use crate::process::pty_session_proof::CapabilityScope;
use crate::process::pty_session_proof::PtySessionProof;
use anyhow::{Result, bail, ensure};

impl CommittedPtyReopenSealAuthority {
    pub(in crate::process) fn validate_missing_proof(&self) -> Result<()> {
        ensure!(
            matches!(
                &self.0,
                ReopenSealAuthority::NotApplicable(NotApplicableReason::MissingProof)
            ),
            REOPEN_SEAL_AUTHORITY_MISMATCH_DETAIL
        );
        Ok(())
    }
}

impl PtySessionProof {
    pub(in crate::process) fn revalidate_committed_reopen_seal(
        &self,
        authority: &CommittedPtyReopenSealAuthority,
    ) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            return self.revalidate_macos_committed_reopen_seal(authority);
        }
        #[cfg(not(target_os = "macos"))]
        match (&authority.0, self.scope) {
            (
                ReopenSealAuthority::NotApplicable(NotApplicableReason::ProcessDiscovery),
                CapabilityScope::ProcessDiscovery,
            )
            | (
                ReopenSealAuthority::NotApplicable(NotApplicableReason::UnsupportedPlatform),
                CapabilityScope::Pty,
            ) => Ok(()),
            _ => bail!(REOPEN_SEAL_AUTHORITY_MISMATCH_DETAIL),
        }
    }

    #[cfg(target_os = "macos")]
    fn revalidate_macos_committed_reopen_seal(
        &self,
        authority: &CommittedPtyReopenSealAuthority,
    ) -> Result<()> {
        use crate::process::pty_session_proof::macos_reopen_seal_state::MacosPtyReopenSealState;
        match (&authority.0, &self.reopen_seal) {
            (ReopenSealAuthority::Installed(identity), MacosPtyReopenSealState::Active(seal))
                if !self.is_process_discovery() && seal.has_identity(identity) =>
            {
                seal.revalidate_committed(self)
            }
            (
                ReopenSealAuthority::NotApplicable(NotApplicableReason::ProcessDiscovery),
                MacosPtyReopenSealState::Idle,
            ) if self.is_process_discovery() => Ok(()),
            _ => bail!(REOPEN_SEAL_AUTHORITY_MISMATCH_DETAIL),
        }
    }
}
