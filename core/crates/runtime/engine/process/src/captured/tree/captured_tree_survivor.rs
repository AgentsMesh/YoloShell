use super::transition::ProcessTransitionError;
use super::{CapturedProcessTree, gone};
use crate::process::{
    DestructiveSessionUnavailable, ProcessIdentity, RootGoneWithoutWait, TrackedProducersGone,
    UnreapedRootExit,
};
use anyhow::Result;

enum EmptyConfirmation {
    Unconfirmed,
    Observed,
}

pub struct TerminationUnsupportedProcessTree {
    pub(super) tree: CapturedProcessTree,
    confirmation: EmptyConfirmation,
    exact_observer_only: bool,
}

pub enum SurvivorReady {
    Pending(bool),
    Gone,
}

pub enum SurvivorPoll {
    Pending(TerminationUnsupportedProcessTree),
    Gone(TrackedProducersGone),
}

impl CapturedProcessTree {
    pub fn into_termination_unsupported(
        self,
        _proof: &DestructiveSessionUnavailable,
    ) -> TerminationUnsupportedProcessTree {
        TerminationUnsupportedProcessTree::new(self)
    }
}

impl super::FreezingProcessTree {
    pub fn into_termination_unsupported(
        self,
        proof: &DestructiveSessionUnavailable,
    ) -> std::result::Result<TerminationUnsupportedProcessTree, ProcessTransitionError<Self>> {
        if self.destructive_started {
            return Err(ProcessTransitionError::new(
                self,
                anyhow::Error::new(proof.clone()),
            ));
        }
        Ok(TerminationUnsupportedProcessTree::new(self.tree))
    }
}

impl TerminationUnsupportedProcessTree {
    fn new(mut tree: CapturedProcessTree) -> Self {
        let exact_observer_only = tree.retire_stale_pty_proof_for_observer();
        Self {
            tree,
            confirmation: EmptyConfirmation::Unconfirmed,
            exact_observer_only,
        }
    }

    pub fn prepare_poll(&mut self) -> Result<SurvivorReady> {
        if self.exact_observer_only {
            return Ok(SurvivorReady::Pending(
                self.tree.live_identities()?.is_empty(),
            ));
        }
        let empty = matches!(gone::observe(&mut self.tree)?, gone::GoneObservation::Empty);
        Ok(
            if empty && matches!(self.confirmation, EmptyConfirmation::Observed) {
                SurvivorReady::Gone
            } else {
                SurvivorReady::Pending(empty)
            },
        )
    }

    pub fn commit_poll(mut self, ready: SurvivorReady) -> SurvivorPoll {
        match ready {
            SurvivorReady::Pending(true) => {
                self.confirmation = EmptyConfirmation::Observed;
                SurvivorPoll::Pending(self)
            }
            SurvivorReady::Pending(false) => {
                self.confirmation = EmptyConfirmation::Unconfirmed;
                SurvivorPoll::Pending(self)
            }
            SurvivorReady::Gone if self.exact_observer_only => {
                self.confirmation = EmptyConfirmation::Unconfirmed;
                SurvivorPoll::Pending(self)
            }
            SurvivorReady::Gone => SurvivorPoll::Gone(gone::complete(self.tree)),
        }
    }

    pub fn root(&self) -> ProcessIdentity {
        self.tree.root()
    }

    pub fn mark_root_exited_unreaped(&mut self, proof: UnreapedRootExit) -> Result<()> {
        self.tree.mark_root_exited_unreaped(proof)
    }

    pub fn mark_root_gone_without_wait(&mut self, proof: RootGoneWithoutWait) -> Result<()> {
        self.tree.mark_root_gone_without_wait(proof)
    }

    pub fn mark_ownership_fault(&mut self) {
        self.tree.mark_ownership_fault();
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn is_exact_observer_only_for_test(&self) -> bool {
        self.exact_observer_only
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn retains_pty_proof_for_test(&self) -> bool {
        self.tree.pty_proof.is_some()
    }
}

impl CapturedProcessTree {
    fn retire_stale_pty_proof_for_observer(&mut self) -> bool {
        if !self.requires_pty_object_lease() {
            return false;
        }
        self.pty_proof = None;
        self.pty_scan.restart();
        self.capability_scan_pending = false;
        self.capability_scheduling_complete = false;
        self.discovery_epoch = Default::default();
        self.discovery_authoritative_empty = false;
        self.reset_discovery_publication_audit();
        true
    }
}
