use super::{
    DirectRetirementReady, MasterReleasePendingProducersGone, ProducersGoneRetirement,
    ProducersGoneRetirementDecision, ProducersGoneRetirementIdentity, ProducersGoneRetirementReady,
    ReportedProducersGone,
};
use crate::process::{
    ProcessIdentity, ProcessTransitionError, RootGoneWithoutWait, UnreapedRootExit,
};
use anyhow::{Context, Result, ensure};
use std::sync::Arc;

const RETIREMENT_AUTHORITY_MISMATCH_DETAIL: &str =
    "producer-retirement authority belongs to another pending closure";
const MASTER_RELEASE_PROOF_REQUIRED_CONTEXT: &str =
    "pending PTY master release has no exact local-and-Host closure proof";
const MASTER_RELEASE_PROOF_REPLAY_DETAIL: &str =
    "PTY master-close proof was replayed after its process closure accepted it";

impl DirectRetirementReady {
    pub fn commit(mut self) -> ReportedProducersGone {
        self.0.tree.retire_session();
        self.0
    }
}

impl MasterReleasePendingProducersGone {
    pub(super) fn new(tree: super::super::CapturedProcessTree) -> Self {
        Self {
            tree,
            retirement_identity: Arc::new(ProducersGoneRetirementIdentity),
            #[cfg(any(test, feature = "runtime-component-tests"))]
            master_release_panic: None,
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

    pub fn prepare_retirement_after_master_release(
        &mut self,
        proof: Option<crate::process::PtyMasterAliasesClosed>,
    ) -> Result<ProducersGoneRetirementReady> {
        if self.tree.pty_master_released() {
            ensure!(proof.is_none(), MASTER_RELEASE_PROOF_REPLAY_DETAIL);
        } else {
            let proof = proof.context(MASTER_RELEASE_PROOF_REQUIRED_CONTEXT)?;
            #[cfg(target_os = "macos")]
            if let Some(session) = self.tree.pty_proof.as_ref() {
                session.validate_reopen_seal_after_master_alias_close()?;
            }
            self.tree.mark_pty_master_released(proof)?;
        }
        #[cfg(target_os = "macos")]
        if let Some(proof) = self.tree.pty_proof.as_mut() {
            proof.complete_reopen_seal_after_master_alias_close()?;
        }
        #[cfg(any(test, feature = "runtime-component-tests"))]
        if let Some(detail) = self.master_release_panic.take() {
            panic!("{detail}");
        }
        #[cfg(target_os = "linux")]
        if !self.tree.settle_linux_after_master_release()? {
            return Ok(self.ready(ProducersGoneRetirementDecision::Refreeze));
        }
        Ok(self.ready(ProducersGoneRetirementDecision::Retire))
    }

    pub fn commit_retirement_after_master_release(
        self,
        ready: ProducersGoneRetirementReady,
    ) -> std::result::Result<ProducersGoneRetirement, ProcessTransitionError<Self>> {
        if !Arc::ptr_eq(&self.retirement_identity, &ready.identity) {
            return Err(ProcessTransitionError::new(
                self,
                anyhow::anyhow!(RETIREMENT_AUTHORITY_MISMATCH_DETAIL),
            ));
        }
        Ok(match ready.decision {
            #[cfg(target_os = "linux")]
            ProducersGoneRetirementDecision::Refreeze => {
                let previous_version = self.tree.closure_version.clone();
                ProducersGoneRetirement::Refreeze(super::super::FreezingProcessTree::enter(
                    self.tree,
                    previous_version,
                    true,
                ))
            }
            ProducersGoneRetirementDecision::Retire => {
                let mut completion = ReportedProducersGone::from_tree(self.tree);
                completion.tree.retire_session();
                ProducersGoneRetirement::Retired(completion)
            }
        })
    }

    fn ready(&self, decision: ProducersGoneRetirementDecision) -> ProducersGoneRetirementReady {
        ProducersGoneRetirementReady {
            identity: Arc::clone(&self.retirement_identity),
            decision,
        }
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn panic_during_master_release_settlement(&mut self, detail: &'static str) {
        self.master_release_panic = Some(detail);
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn session_is_retired(&self) -> bool {
        self.tree.session_is_retired()
    }
}
