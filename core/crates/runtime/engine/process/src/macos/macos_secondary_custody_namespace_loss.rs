use super::MacosSecondaryCustody;
use crate::process::ProcessIdentity;
use crate::process::pty_session_proof::{
    MacosCommittedPtyReopenSeal, MacosFinalizedPtyReopenSeal, MacosNamespaceLossFinalize,
    MacosPtyHolderExclusions, MacosPtyObjectIdentity,
};
use crate::pty_custody::CustodyRuntimeIdentity;
use anyhow::{Result, ensure};

const EOF_SEAL_CHANGED_DETAIL: &str = "namespace-loss PTY reopen seal changed across kernel EOF";

#[must_use]
pub struct MacosPtyKernelEof {
    _object: MacosPtyObjectIdentity,
    _seal: MacosNamespaceLossFinalized,
}

pub struct MacosNamespaceLossContainmentAuthority {
    runtime: CustodyRuntimeIdentity,
    seal: MacosCommittedPtyReopenSeal,
}

#[derive(Clone)]
pub struct MacosNamespaceLossFinalized {
    runtime: CustodyRuntimeIdentity,
    seal: MacosFinalizedPtyReopenSeal,
}

pub enum MacosNamespaceLossSealFinalization {
    Holders(Vec<ProcessIdentity>),
    Finalized(MacosNamespaceLossFinalized),
}

impl MacosSecondaryCustody {
    pub fn namespace_loss_containment_snapshot(
        &self,
    ) -> Result<(MacosNamespaceLossContainmentAuthority, Vec<ProcessIdentity>)> {
        let exclusions = MacosPtyHolderExclusions::from_runtime(&self.identity.runtime);
        let (seal, holders) = self
            .namespace_loss_seal
            .containment_snapshot(&self.pty_master, &exclusions)?;
        Ok((
            MacosNamespaceLossContainmentAuthority {
                runtime: self.identity.runtime.clone(),
                seal,
            },
            holders,
        ))
    }

    pub fn finalize_namespace_loss_reopen_seal(
        &self,
        authority: &MacosNamespaceLossContainmentAuthority,
    ) -> Result<MacosNamespaceLossSealFinalization> {
        ensure!(
            authority.authorizes(&self.identity.runtime),
            EOF_SEAL_CHANGED_DETAIL
        );
        let exclusions = MacosPtyHolderExclusions::from_runtime(&authority.runtime);
        match self.namespace_loss_seal.finalize_if_quiescent(
            &self.pty_master,
            &authority.seal,
            &exclusions,
        )? {
            MacosNamespaceLossFinalize::Holders(holders) => {
                Ok(MacosNamespaceLossSealFinalization::Holders(holders))
            }
            MacosNamespaceLossFinalize::Finalized(seal) => Ok(
                MacosNamespaceLossSealFinalization::Finalized(MacosNamespaceLossFinalized {
                    runtime: self.identity.runtime.clone(),
                    seal,
                }),
            ),
        }
    }

    pub fn drain_namespace_loss_output_to_kernel_eof(
        &self,
        finalized: &MacosNamespaceLossFinalized,
    ) -> Result<Option<MacosPtyKernelEof>> {
        ensure!(
            finalized.runtime == self.identity.runtime,
            EOF_SEAL_CHANGED_DETAIL
        );
        let before = self
            .namespace_loss_seal
            .ensure_committed(&self.pty_master)?;
        let object = self.pty_master.object_identity();
        ensure!(
            finalized.seal.validates(&before, object),
            EOF_SEAL_CHANGED_DETAIL
        );
        if !self.pty_master.drain_to_kernel_eof()? {
            return Ok(None);
        }
        let after = self
            .namespace_loss_seal
            .ensure_committed(&self.pty_master)?;
        ensure!(
            before.validates(&after, object) && finalized.seal.validates(&after, object),
            EOF_SEAL_CHANGED_DETAIL
        );
        Ok(Some(MacosPtyKernelEof {
            _object: object,
            _seal: finalized.clone(),
        }))
    }
}

impl MacosNamespaceLossContainmentAuthority {
    pub fn authorizes(&self, runtime: &CustodyRuntimeIdentity) -> bool {
        &self.runtime == runtime
    }

    pub fn same_authority(&self, other: &Self) -> bool {
        self.runtime == other.runtime && self.seal.same_authority(&other.seal)
    }
}
