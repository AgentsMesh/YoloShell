use super::pty_exit_observer::RootExitObserver;
#[cfg(test)]
use super::pty_fault_reaper::FaultReapCompletion;
use super::pty_fault_reaper::FaultReaperReservation;
use super::pty_output_progress::{PtyOutputRecovery, ReaderBackedPtyOutput};
use super::pty_process_custody::PtyProcessCustody;
#[cfg(test)]
use super::startup_guard::PtyChild;
use crate::process::{CapturedProcessTree, RootWaitAuthority, UnreapedRootExit};
use anyhow::Result;

pub(crate) const OWNERSHIP_TREE_MISSING_DETAIL: &str = "PTY ownership tree is present";
pub(crate) const OWNERSHIP_AUTHORITY_MISSING_DETAIL: &str = "PTY child wait authority is present";
pub(crate) const OUTPUT_PROGRESS_MISSING_DETAIL: &str = "PTY output-progress authority is present";

#[path = "pty_ownership_custody.rs"]
mod custody;
#[path = "pty_fault_cleanup.rs"]
mod fault_cleanup;
#[path = "pty_ownership_handoff_extraction.rs"]
mod handoff_extraction;
#[path = "pty_ownership_missing_eof.rs"]
mod missing_eof;
#[path = "pty_ownership_drop.rs"]
mod ownership_drop;
#[path = "pty_ownership_processes.rs"]
mod processes;
#[path = "pty_reap_outcome.rs"]
mod reap_outcome;
use custody::ArmedPtyOwnership;
pub(crate) use custody::{PtyCustodyAccess, PtyOwnershipCustody};
#[cfg(test)]
pub(crate) use handoff_extraction::HandoffExtractionPhase;
pub(crate) use reap_outcome::{FaultSettlement, ReapEvidence, ReapOutcome};

pub(crate) struct PtyOwnership {
    pub(crate) armed_ownership: Option<ArmedPtyOwnership>,
    pub(crate) reap_evidence: Option<ReapEvidence>,
    pub(crate) armed: bool,
    #[cfg(test)]
    pub(crate) emergency_payload: Option<Box<dyn std::any::Any + Send>>,
    #[cfg(test)]
    pub(crate) handoff_extraction_panic: Option<handoff_extraction::HandoffExtractionPhase>,
}

#[cfg(test)]
pub(crate) fn reserve_fault_reaper() -> Result<FaultReaperReservation> {
    FaultReaperReservation::reserve()
}

impl PtyOwnership {
    pub(crate) fn from_authority(
        authority: RootWaitAuthority,
        tree: CapturedProcessTree,
        output_progress: ReaderBackedPtyOutput,
        fault_reaper: FaultReaperReservation,
    ) -> Self {
        Self {
            armed_ownership: Some(ArmedPtyOwnership::new(
                PtyOwnershipCustody::tracking(authority, tree, output_progress),
                fault_reaper,
            )),
            reap_evidence: None,
            armed: true,
            #[cfg(test)]
            emergency_payload: None,
            #[cfg(test)]
            handoff_extraction_panic: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(
        child: PtyChild,
        tree: CapturedProcessTree,
        fault_reaper: FaultReaperReservation,
    ) -> Result<Self> {
        Self::new_for_test_with_output(child, tree, fault_reaper, ReaderBackedPtyOutput::for_test())
    }

    #[cfg(test)]
    pub(crate) fn new_for_test_with_output(
        child: PtyChild,
        tree: CapturedProcessTree,
        fault_reaper: FaultReaperReservation,
        output: ReaderBackedPtyOutput,
    ) -> Result<Self> {
        let binding = RootWaitAuthority::prove_owned_child(&child, &tree)?;
        let authority = binding.bind(child);
        Ok(Self::from_authority(authority, tree, output, fault_reaper))
    }

    pub(crate) fn reap_evidence(&self) -> Option<ReapEvidence> {
        self.reap_evidence
    }

    pub(crate) fn wait_until_reaped(&mut self) -> ReapOutcome {
        debug_assert!(self.reap_evidence.is_none());
        debug_assert!(
            self.custody()
                .processes
                .as_ref()
                .is_some_and(PtyProcessCustody::session_is_retired)
        );
        let custody = self.custody_mut();
        let outcome = match (custody.authority.take(), custody.root_exit_proof.take()) {
            (Some(authority), Some(proof)) => match authority.reap(proof) {
                Ok(status) => ReapOutcome::waited(Some(status)),
                Err(error) => {
                    let mut outcome = ReapOutcome::lost_unknown();
                    outcome.fault = Some(error);
                    outcome
                }
            },
            _ => {
                let mut outcome = ReapOutcome::lost_unknown();
                outcome.fault = Some(anyhow::anyhow!(OWNERSHIP_AUTHORITY_MISSING_DETAIL));
                outcome
            }
        };
        self.reap_evidence = Some(outcome.evidence);
        outcome
    }

    pub(crate) fn poll_root_exit(
        &self,
        observer: &mut dyn RootExitObserver,
    ) -> Result<Option<UnreapedRootExit>> {
        observer.poll_exited_unreaped(
            self.custody()
                .authority
                .as_ref()
                .expect(OWNERSHIP_AUTHORITY_MISSING_DETAIL),
        )
    }

    pub(crate) fn output_recovery(&self) -> PtyOutputRecovery {
        self.custody()
            .output_progress
            .reader()
            .expect(OUTPUT_PROGRESS_MISSING_DETAIL)
            .recovery()
    }

    pub(crate) fn root_exit_requires_output_progress(&self) -> Result<bool> {
        #[cfg(target_os = "macos")]
        return crate::process::macos_native::process_exit_state(
            self.custody()
                .authority
                .as_ref()
                .expect(OWNERSHIP_AUTHORITY_MISSING_DETAIL)
                .root()
                .pid,
        )
        .map(crate::process::macos_native::ProcessExitState::requires_output_progress);
        #[cfg(not(target_os = "macos"))]
        Ok(false)
    }

    pub(crate) fn disarm(&mut self) {
        debug_assert!(self.reap_evidence.is_some());
        debug_assert!(
            self.custody()
                .processes
                .as_ref()
                .is_some_and(PtyProcessCustody::session_is_retired)
        );
        self.armed = false;
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/ownership/pty_ownership_cleanup_tests.rs"]
mod cleanup_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/ownership/pty_ownership_drop_containment_tests.rs"]
mod drop_containment_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/ownership/pty_ownership_drop_nonblocking_tests.rs"]
mod drop_nonblocking_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/ownership/pty_ownership_handoff_extraction_tests.rs"]
mod handoff_extraction_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/ownership/pty_ownership_handoff_tests.rs"]
mod handoff_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/ownership/pty_ownership_observer_handoff_tests.rs"]
mod observer_handoff_tests;
