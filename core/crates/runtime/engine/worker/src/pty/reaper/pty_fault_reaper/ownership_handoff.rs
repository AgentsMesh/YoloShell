use super::ownership_handoff_transaction::OwnershipHandoffTransaction;
use super::{FaultReapCompletion, FaultReaperReservation};
use crate::worker::pty_ownership::PtyOwnershipCustody;

pub(crate) struct OwnershipFaultHandoff {
    pub(crate) custody: PtyOwnershipCustody,
    pub(crate) completion: FaultReapCompletion,
    pub(crate) failure: Option<anyhow::Error>,
}

impl OwnershipFaultHandoff {
    pub(crate) fn new(custody: PtyOwnershipCustody, completion: FaultReapCompletion) -> Self {
        Self {
            custody,
            completion,
            failure: None,
        }
    }

    pub(crate) fn with_failure(
        custody: PtyOwnershipCustody,
        completion: FaultReapCompletion,
        failure: Option<anyhow::Error>,
    ) -> Self {
        Self {
            custody,
            completion,
            failure,
        }
    }

    pub(crate) fn prepare(&mut self) {
        if let Some(processes) = self.custody.processes.as_mut() {
            super::fault_reap_construction::prepare_processes(processes);
        }
    }

    pub(crate) fn custody(&self) -> &PtyOwnershipCustody {
        &self.custody
    }

    pub(crate) fn custody_mut(&mut self) -> &mut PtyOwnershipCustody {
        &mut self.custody
    }
}

impl FaultReaperReservation {
    #[cfg(test)]
    pub(crate) fn panic_on_handoff_build_for_test(&mut self) {
        assert!(!self.handoff_build_panics);
        self.handoff_build_panics = true;
    }

    #[cfg(test)]
    pub(crate) fn panic_on_handoff_dispatch_for_test(&mut self) {
        assert!(!self.handoff_dispatch_panics);
        self.handoff_dispatch_panics = true;
    }

    #[cfg(test)]
    pub(crate) fn take_handoff_build_panic_for_test(&mut self) -> bool {
        std::mem::take(&mut self.handoff_build_panics)
    }

    #[cfg(test)]
    pub(crate) fn take_handoff_dispatch_panic_for_test(&mut self) -> bool {
        std::mem::take(&mut self.handoff_dispatch_panics)
    }

    pub(crate) fn prepare_ownership_handoff(&mut self) -> OwnershipHandoffTransaction {
        OwnershipHandoffTransaction::prepare(self)
    }
}
