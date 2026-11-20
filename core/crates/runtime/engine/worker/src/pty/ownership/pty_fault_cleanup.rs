use super::{FaultSettlement, PtyOwnership, ReapOutcome};
use crate::worker::pty::pty_fault_reaper::FaultReapCompletion;
use anyhow::anyhow;

pub(crate) const DEFERRED_REAP_DETAIL: &str = "PTY child reap was handed to the fault reaper";
pub(crate) const OWNERSHIP_CHILD_MISSING_DETAIL: &str = "PTY ownership child is present";
pub(crate) const OWNERSHIP_PROCESSES_MISSING_DETAIL: &str = "PTY process custody is present";
pub(crate) const FAULT_REAPER_MISSING_DETAIL: &str = "PTY fault reaper reservation is present";

impl PtyOwnership {
    pub(crate) fn settle_after_fault(
        &mut self,
        completion: FaultReapCompletion,
    ) -> FaultSettlement {
        self.settle_after_fault_with_evidence(completion, None)
    }

    pub(crate) fn settle_after_fault_with_failure(
        &mut self,
        completion: FaultReapCompletion,
        failure: anyhow::Error,
    ) -> FaultSettlement {
        self.settle_after_fault_with_evidence(completion, Some(failure))
    }

    pub(crate) fn settle_after_fault_with_evidence(
        &mut self,
        completion: FaultReapCompletion,
        failure: Option<anyhow::Error>,
    ) -> FaultSettlement {
        if let Some(evidence) = self.reap_evidence() {
            let mut outcome = ReapOutcome::without_status(evidence);
            outcome.fault = failure;
            return FaultSettlement::Reaped(outcome);
        }
        self.defer_to_reaper(completion, failure)
    }

    pub(crate) fn defer_to_reaper(
        &mut self,
        completion: FaultReapCompletion,
        failure: Option<anyhow::Error>,
    ) -> FaultSettlement {
        self.stage_fault_handoff(completion, failure);
        self.assert_fault_handoff_ready();
        super::handoff_extraction::OwnershipHandoffExtraction::capture(self).install();
        self.armed = false;
        FaultSettlement::Deferred(anyhow!(DEFERRED_REAP_DETAIL))
    }

    pub(crate) fn stage_fault_handoff(
        &mut self,
        completion: FaultReapCompletion,
        failure: Option<anyhow::Error>,
    ) {
        let armed = self
            .armed_ownership
            .as_mut()
            .expect(FAULT_REAPER_MISSING_DETAIL);
        if armed.fault_completion.is_none() {
            armed.fault_completion = Some(completion);
        } else {
            let _ = super::super::supervision::contain(|| drop(completion));
        }
        let Some(failure) = failure else { return };
        if armed.fault.is_none() {
            armed.fault = Some(failure);
        } else {
            let _ = super::super::supervision::contain(|| drop(failure));
        }
    }

    pub(crate) fn assert_fault_handoff_ready(&self) {
        let custody = self.custody();
        assert!(
            custody.authority.is_some(),
            "{OWNERSHIP_CHILD_MISSING_DETAIL}"
        );
        assert!(
            custody.processes.is_some(),
            "{OWNERSHIP_PROCESSES_MISSING_DETAIL}"
        );
        assert!(
            custody.output_progress.reader().is_some(),
            "{}",
            super::OUTPUT_PROGRESS_MISSING_DETAIL
        );
        assert!(
            self.armed_ownership.is_some(),
            "{FAULT_REAPER_MISSING_DETAIL}"
        );
    }
}
