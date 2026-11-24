use super::CapturedProcessTree;
use super::ownership::SessionOwnership;
use crate::process::inventory::SessionOwnershipProof;
use crate::process::{RootGoneWithoutWait, UnreapedRootExit};
use anyhow::{Result, ensure};

const ROOT_EXIT_PROOF_MISMATCH_DETAIL: &str = "root-exit proof belongs to another process";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RootExitState {
    Live,
    ExitedUnreaped,
    GoneWithoutWait,
}

impl RootExitState {
    pub(super) fn is_gone(self) -> bool {
        self != Self::Live
    }
}

impl CapturedProcessTree {
    pub(crate) fn reconcile_retained_root_exit(&mut self) -> Result<()> {
        if self.root_exit == RootExitState::GoneWithoutWait {
            return Ok(());
        }
        let root = self.root();
        let live_root = self.live_root();
        if let Some(proof) = RootGoneWithoutWait::observe_exact_generation(root, &live_root)? {
            return self.mark_root_gone_without_wait(proof);
        }
        #[cfg(target_os = "macos")]
        if self.root_exit == RootExitState::Live
            && let Some(proof) = self.events.exact_root_exit_proof(root, live_root)
        {
            return self.mark_root_exited_unreaped(proof);
        }
        Ok(())
    }

    pub(crate) fn reconcile_retained_exact_empty(&mut self) -> Result<()> {
        if self.pty_proof.is_some()
            || !self.root_exit.is_gone()
            || !self.captured_targets().is_empty()
            || !self.lineage_settled
            || !self.events.is_gated()
            || !self.events.inventory_publication_is_current()
            || !self.events.exact_closure_isolated(&[])
        {
            return Ok(());
        }
        self.events.acknowledge_root_exit_discovery_epoch();
        self.refresh()?;
        Ok(())
    }

    pub fn mark_root_exited_unreaped(&mut self, proof: UnreapedRootExit) -> Result<()> {
        self.validate_root_proof(proof.identities())?;
        if self.ownership == SessionOwnership::Retired {
            return Ok(());
        }
        if let Some(proof) = self.pty_proof.as_ref() {
            proof.confirm_process_capability_root_exit(&self.live_root())?;
        }
        self.events.confirm_root_exit(self.live_root());
        let first_exit = !self.root_exit.is_gone();
        if self.root_exit != RootExitState::GoneWithoutWait {
            self.root_exit = RootExitState::ExitedUnreaped;
        }
        if first_exit {
            self.begin_root_exit_discovery_epoch();
        }
        Ok(())
    }

    pub fn mark_root_gone_without_wait(&mut self, proof: RootGoneWithoutWait) -> Result<()> {
        self.validate_root_proof(proof.identities())?;
        if self.ownership != SessionOwnership::Retired
            && let Some(proof) = self.pty_proof.as_ref()
        {
            proof.confirm_process_capability_root_exit(&self.live_root())?;
        }
        self.events.confirm_root_exit(self.live_root());
        let first_exit = !self.root_exit.is_gone();
        self.root_exit = RootExitState::GoneWithoutWait;
        if first_exit && self.ownership != SessionOwnership::Retired {
            self.begin_root_exit_discovery_epoch();
        }
        Ok(())
    }

    pub(super) fn root_ownership_proof(&self) -> SessionOwnershipProof {
        match (self.root_exit, self.ownership) {
            (_, SessionOwnership::Retired) => SessionOwnershipProof::SessionRetired,
            (RootExitState::ExitedUnreaped, _) => SessionOwnershipProof::RootExitedUnreaped,
            (RootExitState::GoneWithoutWait, _) => SessionOwnershipProof::RootGoneWithoutWait,
            (RootExitState::Live, SessionOwnership::Tracking) => SessionOwnershipProof::LiveWitness,
            (RootExitState::Live, SessionOwnership::OwnershipFault) => {
                SessionOwnershipProof::SessionScanDisabled
            }
        }
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn root_exit_proof_for_test(
        &self,
        root: crate::process::ProcessIdentity,
    ) -> UnreapedRootExit {
        UnreapedRootExit::for_test(root, self.live_root())
    }

    fn validate_root_proof(
        &self,
        identities: (
            crate::process::ProcessIdentity,
            crate::process::live_identity::LiveProcessIdentityKey,
        ),
    ) -> Result<()> {
        ensure!(
            identities == (self.root(), self.live_root().key()),
            ROOT_EXIT_PROOF_MISMATCH_DETAIL
        );
        Ok(())
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/root/captured_tree_root_exit_tests.rs"]
mod tests;

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/root/captured_tree_root_exit_discovery_tests.rs"]
mod discovery_tests;
