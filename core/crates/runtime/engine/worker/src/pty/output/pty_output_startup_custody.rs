use super::{ExclusivePtyOutput, OutputDescriptor};
use crate::process::{
    PtyHostReleaseVerifier, PtyLocalMasterAliasesClosed, PtyMasterAliasReleaseAuthority,
    PtyMasterAliasesClosed,
};
use crate::pty_custody::{CustodyHolderIdentity, ExactCustodyReleasedAck};
use crate::worker::startup::startup_custody_transfer::StartupCustodyTransferFence;
use anyhow::{Result, bail, ensure};
use std::sync::Arc;
#[path = "pty_output_startup_custody_release.rs"]
mod release;
pub(crate) const HOST_RELEASE_ALREADY_REQUIRED_DETAIL: &str =
    "startup PTY output already has secondary custody";
pub(crate) const HOST_RELEASE_NOT_REQUIRED_DETAIL: &str =
    "startup PTY output has no pending Host custody release";
pub(crate) const MASTER_ALIAS_AUTHORITY_UNAVAILABLE_CONTEXT: &str =
    "PTY master-alias closure authority is unavailable";
pub(crate) enum StartupCustodyReleaseState {
    NoSecondaryCustody,
    TransferPending(Arc<StartupCustodyTransferFence>),
    HostReleaseRequired(CustodyHolderIdentity),
    LocalClosedPendingHostAck {
        local: PtyLocalMasterAliasesClosed,
        verifier: PtyHostReleaseVerifier,
        holder: CustodyHolderIdentity,
    },
    HostReleaseProven(PtyMasterAliasesClosed),
    ProofDelivered,
}

impl ExclusivePtyOutput {
    pub(crate) fn bind_custody_transfer_fence(
        &mut self,
        fence: Arc<StartupCustodyTransferFence>,
    ) -> Result<()> {
        ensure!(
            matches!(
                self.startup_custody,
                StartupCustodyReleaseState::NoSecondaryCustody
            ),
            HOST_RELEASE_ALREADY_REQUIRED_DETAIL
        );
        self.startup_custody = StartupCustodyReleaseState::TransferPending(fence);
        Ok(())
    }

    pub(crate) fn require_host_release(&mut self, holder: CustodyHolderIdentity) -> Result<()> {
        let valid = match &self.startup_custody {
            StartupCustodyReleaseState::NoSecondaryCustody => true,
            StartupCustodyReleaseState::TransferPending(fence) => fence.holder() == Some(holder),
            _ => false,
        };
        ensure!(valid, HOST_RELEASE_ALREADY_REQUIRED_DETAIL);
        self.startup_custody = StartupCustodyReleaseState::HostReleaseRequired(holder);
        Ok(())
    }

    pub(crate) fn close_local_aliases_for_host_release(&mut self) -> Result<()> {
        let StartupCustodyReleaseState::HostReleaseRequired(holder) = self.startup_custody else {
            bail!(HOST_RELEASE_NOT_REQUIRED_DETAIL);
        };
        let (local, verifier) = self.close_local_aliases()?;
        self.startup_custody = StartupCustodyReleaseState::LocalClosedPendingHostAck {
            local,
            verifier,
            holder,
        };
        Ok(())
    }

    pub(crate) fn prove_host_release(&mut self, exact: ExactCustodyReleasedAck) -> Result<()> {
        let state = std::mem::replace(
            &mut self.startup_custody,
            StartupCustodyReleaseState::ProofDelivered,
        );
        let StartupCustodyReleaseState::LocalClosedPendingHostAck {
            local,
            verifier,
            holder,
        } = state
        else {
            self.startup_custody = state;
            bail!(HOST_RELEASE_NOT_REQUIRED_DETAIL);
        };
        let acknowledgement = verifier.clone().acknowledge(exact);
        match local.finalize_host_release(acknowledgement) {
            Ok(settlement) => {
                self.startup_custody =
                    StartupCustodyReleaseState::HostReleaseProven(settlement.mint_final_proof());
                Ok(())
            }
            Err((local, error)) => {
                self.startup_custody = StartupCustodyReleaseState::LocalClosedPendingHostAck {
                    local,
                    verifier,
                    holder,
                };
                Err(error)
            }
        }
    }

    pub(crate) fn close_local_aliases(
        &mut self,
    ) -> Result<(PtyLocalMasterAliasesClosed, PtyHostReleaseVerifier)> {
        self.descriptor = OutputDescriptor::Released;
        self.startup_alias_authority()?.take_local_aliases_closed()
    }

    pub(crate) fn startup_alias_authority(&self) -> Result<Arc<PtyMasterAliasReleaseAuthority>> {
        self.master_alias_authority
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!(MASTER_ALIAS_AUTHORITY_UNAVAILABLE_CONTEXT))
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/output/pty_output_startup_custody_tests.rs"]
mod tests;
