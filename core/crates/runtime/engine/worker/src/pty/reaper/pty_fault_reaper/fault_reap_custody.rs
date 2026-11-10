use super::{FaultReap, FaultReapCompletion, OwnershipFaultHandoff};
use crate::worker::pty_ownership::{PtyCustodyAccess, PtyOwnershipCustody};

pub(crate) const FAULT_HANDOFF_MISSING_DETAIL: &str = "fault-reaper ownership handoff is installed";

impl FaultReap {
    pub(crate) fn handoff(&self) -> &OwnershipFaultHandoff {
        self.handoff.as_ref().expect(FAULT_HANDOFF_MISSING_DETAIL)
    }

    pub(crate) fn handoff_mut(&mut self) -> &mut OwnershipFaultHandoff {
        self.handoff.as_mut().expect(FAULT_HANDOFF_MISSING_DETAIL)
    }

    pub(crate) fn custody(&self) -> &PtyOwnershipCustody {
        self.handoff().custody()
    }

    pub(crate) fn custody_mut(&mut self) -> &mut PtyOwnershipCustody {
        self.handoff_mut().custody_mut()
    }

    pub(crate) fn completion(&self) -> &FaultReapCompletion {
        &self.handoff().completion
    }

    #[cfg(test)]
    pub(crate) fn replace_completion_for_test(&mut self, completion: FaultReapCompletion) {
        self.handoff_mut().completion = completion;
    }

    pub(crate) fn access(&self) -> PtyCustodyAccess {
        self.custody().access()
    }

    pub(crate) fn is_observer_only(&self) -> bool {
        self.custody().is_observer_only()
    }
}
