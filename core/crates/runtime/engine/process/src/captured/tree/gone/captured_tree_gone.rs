use super::CapturedProcessTree;
use super::transition::ProcessTransitionError;
use crate::process::{
    DescendantObservation, ProcessIdentity, RootGoneWithoutWait, UnreapedRootExit,
};
use anyhow::Result;
use std::sync::Arc;

pub struct TrackedProducersGone {
    tree: CapturedProcessTree,
}

pub struct ReportedProducersGone {
    tree: CapturedProcessTree,
    retirement_identity: Arc<ReportedProducersGoneRetirementIdentity>,
}

pub struct MasterReleasePendingProducersGone {
    tree: CapturedProcessTree,
    retirement_identity: Arc<ProducersGoneRetirementIdentity>,
    #[cfg(any(test, feature = "runtime-component-tests"))]
    master_release_panic: Option<&'static str>,
}

pub struct DirectRetirementReady(ReportedProducersGone);

pub enum ReportedProducersGoneRetirement {
    Direct(DirectRetirementReady),
    MasterRelease(MasterReleasePendingProducersGone),
}

pub struct ReportedProducersGoneRetirementReady {
    identity: Arc<ReportedProducersGoneRetirementIdentity>,
    decision: ReportedProducersGoneRetirementDecision,
}

enum ReportedProducersGoneRetirementDecision {
    Direct,
    MasterRelease,
}

pub enum ProducersGoneRetirement {
    Refreeze(super::FreezingProcessTree),
    Retired(ReportedProducersGone),
}

pub struct ProducersGoneRetirementReady {
    identity: Arc<ProducersGoneRetirementIdentity>,
    decision: ProducersGoneRetirementDecision,
}

enum ProducersGoneRetirementDecision {
    #[cfg(target_os = "linux")]
    Refreeze,
    Retire,
}

struct ProducersGoneRetirementIdentity;

struct ReportedProducersGoneRetirementIdentity;

pub struct ProducersGoneReportReady<Output>(Output);

pub(super) enum GoneObservation {
    Empty,
    NonEmpty,
    Indeterminate,
    CapabilityScanPending,
    OutputFencePending,
}

#[path = "captured_tree_gone_retirement.rs"]
mod retirement;
#[path = "captured_tree_gone_retirement_prepare.rs"]
mod retirement_prepare;

pub(super) fn complete(tree: CapturedProcessTree) -> TrackedProducersGone {
    TrackedProducersGone { tree }
}

pub(super) fn observe(tree: &mut CapturedProcessTree) -> Result<GoneObservation> {
    #[cfg(target_os = "linux")]
    if tree.linux_containment_available() {
        return Ok(match tree.linux_frozen_closure(false)? {
            Some(closure) if closure.is_empty() => GoneObservation::Empty,
            Some(_) => GoneObservation::NonEmpty,
            None => GoneObservation::Indeterminate,
        });
    }
    tree.refresh()?;
    match tree.descendant_observation() {
        DescendantObservation::Indeterminate if !tree.live_descendants()?.is_empty() => {
            Ok(GoneObservation::NonEmpty)
        }
        DescendantObservation::Indeterminate
            if tree.empty_output_fence_can_reconcile_uncertainty() =>
        {
            Ok(GoneObservation::OutputFencePending)
        }
        DescendantObservation::Indeterminate => Ok(GoneObservation::Indeterminate),
        DescendantObservation::CapabilityScanPending if !tree.live_descendants()?.is_empty() => {
            Ok(GoneObservation::NonEmpty)
        }
        DescendantObservation::CapabilityScanPending => Ok(GoneObservation::CapabilityScanPending),
        DescendantObservation::Settled if tree.live_identities()?.is_empty() => {
            Ok(GoneObservation::Empty)
        }
        DescendantObservation::Settled => Ok(GoneObservation::NonEmpty),
    }
}

impl TrackedProducersGone {
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

    pub fn report<Output>(
        mut self,
        operation: impl FnOnce(&Self) -> Result<Output>,
    ) -> std::result::Result<(ReportedProducersGone, Output), ProcessTransitionError<Self>> {
        let ready = match self.prepare_report(operation) {
            Ok(ready) => ready,
            Err(error) => return Err(ProcessTransitionError::new(self, error)),
        };
        Ok(self.commit_report(ready))
    }

    pub fn prepare_report<Output>(
        &mut self,
        operation: impl FnOnce(&Self) -> Result<Output>,
    ) -> Result<ProducersGoneReportReady<Output>> {
        operation(self).map(ProducersGoneReportReady)
    }

    pub fn commit_report<Output>(
        self,
        ready: ProducersGoneReportReady<Output>,
    ) -> (ReportedProducersGone, Output) {
        (ReportedProducersGone::from_tree(self.tree), ready.0)
    }
}

impl ReportedProducersGone {
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

    pub fn session_is_retired(&self) -> bool {
        self.tree.session_is_retired()
    }
}
