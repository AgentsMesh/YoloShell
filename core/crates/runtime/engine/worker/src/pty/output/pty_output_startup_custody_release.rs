use super::{StartupCustodyReleaseState, StartupCustodyTransferFence};
use crate::process::{
    PtyHostReleaseVerifier, PtyLocalMasterAliasesClosed, PtyMasterAliasReleaseAuthority,
};
use crate::pty_custody::CustodyHolderIdentity;
use crate::worker::pty_output_progress::{
    ExclusivePtyOutput, MasterAliasReleasePoll, OutputDescriptor,
};
use anyhow::Result;
use std::sync::Arc;

impl ExclusivePtyOutput {
    pub(crate) fn release_startup_master_aliases(
        &mut self,
        authority: &Arc<PtyMasterAliasReleaseAuthority>,
    ) -> Result<MasterAliasReleasePoll> {
        let state = std::mem::replace(
            &mut self.startup_custody,
            StartupCustodyReleaseState::ProofDelivered,
        );
        match state {
            StartupCustodyReleaseState::NoSecondaryCustody => self.release_local_only_or_restore(
                authority,
                StartupCustodyReleaseState::NoSecondaryCustody,
            ),
            StartupCustodyReleaseState::TransferPending(fence) => {
                self.release_ambiguous_transfer(authority, fence)
            }
            StartupCustodyReleaseState::HostReleaseRequired(holder) => self
                .begin_pending_or_restore(
                    authority,
                    holder,
                    StartupCustodyReleaseState::HostReleaseRequired(holder),
                ),
            StartupCustodyReleaseState::LocalClosedPendingHostAck {
                local,
                verifier,
                holder,
            } => self.poll_holder_gone(local, verifier, holder),
            StartupCustodyReleaseState::HostReleaseProven(proof) => {
                Ok(MasterAliasReleasePoll::Closed(proof))
            }
            StartupCustodyReleaseState::ProofDelivered => Ok(MasterAliasReleasePoll::Delivered),
        }
    }

    pub(crate) fn release_ambiguous_transfer(
        &mut self,
        authority: &Arc<PtyMasterAliasReleaseAuthority>,
        fence: Arc<StartupCustodyTransferFence>,
    ) -> Result<MasterAliasReleasePoll> {
        match fence.holder() {
            Some(holder) => self.begin_pending_or_restore(
                authority,
                holder,
                StartupCustodyReleaseState::TransferPending(fence),
            ),
            None => self.release_local_only_or_restore(
                authority,
                StartupCustodyReleaseState::TransferPending(fence),
            ),
        }
    }

    pub(crate) fn begin_pending_or_restore(
        &mut self,
        authority: &Arc<PtyMasterAliasReleaseAuthority>,
        holder: CustodyHolderIdentity,
        restore: StartupCustodyReleaseState,
    ) -> Result<MasterAliasReleasePoll> {
        self.descriptor = OutputDescriptor::Released;
        match authority.take_local_aliases_closed() {
            Ok((local, verifier)) => self.poll_holder_gone(local, verifier, holder),
            Err(error) => {
                self.startup_custody = restore;
                Err(error)
            }
        }
    }

    pub(crate) fn poll_holder_gone(
        &mut self,
        local: PtyLocalMasterAliasesClosed,
        verifier: PtyHostReleaseVerifier,
        holder: CustodyHolderIdentity,
    ) -> Result<MasterAliasReleasePoll> {
        let Some(gone) = holder.prove_gone() else {
            self.restore_pending(local, verifier, holder);
            return Ok(MasterAliasReleasePoll::Pending);
        };
        let acknowledgement = verifier.clone().acknowledge_holder_gone(gone);
        match local.finalize_host_release(acknowledgement) {
            Ok(settlement) => Ok(MasterAliasReleasePoll::Closed(
                settlement.mint_final_proof(),
            )),
            Err((local, error)) => {
                self.restore_pending(local, verifier, holder);
                Err(error)
            }
        }
    }

    pub(crate) fn release_local_only_or_restore(
        &mut self,
        authority: &Arc<PtyMasterAliasReleaseAuthority>,
        restore: StartupCustodyReleaseState,
    ) -> Result<MasterAliasReleasePoll> {
        self.descriptor = OutputDescriptor::Released;
        match authority.take_local_aliases_closed() {
            Ok((local, _verifier)) => Ok(MasterAliasReleasePoll::Closed(
                local
                    .finalize_without_secondary_custody()
                    .mint_final_proof(),
            )),
            Err(error) => {
                self.startup_custody = restore;
                Err(error)
            }
        }
    }

    pub(crate) fn restore_pending(
        &mut self,
        local: PtyLocalMasterAliasesClosed,
        verifier: PtyHostReleaseVerifier,
        holder: CustodyHolderIdentity,
    ) {
        self.startup_custody = StartupCustodyReleaseState::LocalClosedPendingHostAck {
            local,
            verifier,
            holder,
        };
    }
}
