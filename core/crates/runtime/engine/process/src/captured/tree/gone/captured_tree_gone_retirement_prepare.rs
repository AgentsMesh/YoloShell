use super::{
    DirectRetirementReady, MasterReleasePendingProducersGone, ReportedProducersGone,
    ReportedProducersGoneRetirement, ReportedProducersGoneRetirementDecision,
    ReportedProducersGoneRetirementIdentity, ReportedProducersGoneRetirementReady,
};
use crate::process::{CapturedProcessTree, ProcessTransitionError};
use anyhow::Result;
use std::sync::Arc;

#[cfg(any(test, feature = "runtime-component-tests"))]
const DIRECT_RETIREMENT_REQUIRED_DETAIL: &str = "retained producer custody cannot retire directly";
const RETIREMENT_AUTHORITY_MISMATCH_DETAIL: &str =
    "reported producer-retirement authority belongs to another process closure";

impl ReportedProducersGone {
    pub(super) fn from_tree(tree: CapturedProcessTree) -> Self {
        Self {
            tree,
            retirement_identity: Arc::new(ReportedProducersGoneRetirementIdentity),
        }
    }

    pub fn prepare_retirement(&mut self) -> Result<ReportedProducersGoneRetirementReady> {
        #[cfg(target_os = "macos")]
        self.tree.prepare_local_alias_release()?;
        Ok(ReportedProducersGoneRetirementReady {
            identity: Arc::clone(&self.retirement_identity),
            decision: self.retirement_decision(),
        })
    }

    pub fn into_retirement(
        self,
        ready: ReportedProducersGoneRetirementReady,
    ) -> std::result::Result<ReportedProducersGoneRetirement, ProcessTransitionError<Self>> {
        if !Arc::ptr_eq(&self.retirement_identity, &ready.identity) {
            return Err(ProcessTransitionError::new(
                self,
                anyhow::anyhow!(RETIREMENT_AUTHORITY_MISMATCH_DETAIL),
            ));
        }
        Ok(match ready.decision {
            ReportedProducersGoneRetirementDecision::Direct => {
                ReportedProducersGoneRetirement::Direct(DirectRetirementReady(self))
            }
            ReportedProducersGoneRetirementDecision::MasterRelease => {
                ReportedProducersGoneRetirement::MasterRelease(
                    MasterReleasePendingProducersGone::new(self.tree),
                )
            }
        })
    }

    fn retirement_decision(&self) -> ReportedProducersGoneRetirementDecision {
        #[cfg(target_os = "macos")]
        if self.tree.pty_local_alias_release_prepared {
            return ReportedProducersGoneRetirementDecision::MasterRelease;
        }
        #[cfg(target_os = "linux")]
        if self.tree.linux_containment_available()
            || self.tree.destructive.linux_containment_was_removed()
        {
            return ReportedProducersGoneRetirementDecision::MasterRelease;
        }
        ReportedProducersGoneRetirementDecision::Direct
    }

    #[cfg(all(test, target_os = "macos"))]
    pub(crate) fn retains_local_object_lease_for_test(&self) -> bool {
        self.tree
            .pty_proof
            .as_ref()
            .is_some_and(super::super::PtySessionProof::has_required_pty_object_lease)
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn retire_direct_for_test(mut self) -> Self {
        let ready = self.prepare_retirement().unwrap();
        match self.into_retirement(ready).unwrap() {
            ReportedProducersGoneRetirement::Direct(ready) => ready.commit(),
            ReportedProducersGoneRetirement::MasterRelease(_) => {
                panic!("{DIRECT_RETIREMENT_REQUIRED_DETAIL}")
            }
        }
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn into_master_release_for_test(self) -> MasterReleasePendingProducersGone {
        MasterReleasePendingProducersGone::new(self.tree)
    }
}

#[cfg(target_os = "macos")]
impl CapturedProcessTree {
    fn prepare_local_alias_release(&mut self) -> Result<()> {
        if self.pty_local_alias_release_prepared {
            return Ok(());
        }
        let Some(proof) = self.pty_proof.as_mut() else {
            return Ok(());
        };
        if proof.master_alias_release_requirement()? {
            proof.release_local_object_lease()?;
            self.pty_local_alias_release_prepared = true;
        } else {
            proof.complete_reopen_seal_without_master_aliases()?;
        }
        Ok(())
    }
}
