use super::PtyOwnership;
use crate::process::{CapturedProcessTree, RootWaitAuthority, UnreapedRootExit};
use crate::worker::pty::pty_fault_reaper::FaultOutput;
use crate::worker::pty_force_termination::PtyForceTermination;
use crate::worker::pty_output_progress::ReaderBackedPtyOutput;
use crate::worker::pty_process_custody::PtyProcessCustody;

pub(crate) const OWNERSHIP_CUSTODY_MISSING_DETAIL: &str = "PTY ownership custody is present";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PtyCustodyAccess {
    Control,
    ObserverOnly,
}

impl PtyCustodyAccess {
    pub(crate) fn is_observer_only(self) -> bool {
        matches!(self, Self::ObserverOnly)
    }
}

pub(crate) struct PtyOwnershipCustody {
    pub(crate) authority: Option<RootWaitAuthority>,
    pub(crate) processes: Option<PtyProcessCustody>,
    pub(crate) force: PtyForceTermination,
    pub(crate) output_progress: FaultOutput,
    pub(crate) root_exit_proof: Option<UnreapedRootExit>,
    pub(crate) access: PtyCustodyAccess,
    pub(crate) missing_eof: crate::worker::pty_missing_eof::MissingEofDeadline,
}

pub(crate) struct ArmedPtyOwnership {
    pub(crate) custody: Option<PtyOwnershipCustody>,
    pub(crate) fault_completion: Option<crate::worker::pty::pty_fault_reaper::FaultReapCompletion>,
    pub(crate) fault: Option<anyhow::Error>,
    pub(crate) fault_reaper: crate::worker::pty::pty_fault_reaper::FaultReaperReservation,
}

impl ArmedPtyOwnership {
    pub(crate) fn new(
        custody: PtyOwnershipCustody,
        fault_reaper: crate::worker::pty::pty_fault_reaper::FaultReaperReservation,
    ) -> Self {
        Self {
            custody: Some(custody),
            fault_completion: None,
            fault: None,
            fault_reaper,
        }
    }
}

impl PtyOwnershipCustody {
    pub(crate) fn tracking(
        authority: RootWaitAuthority,
        tree: CapturedProcessTree,
        output: ReaderBackedPtyOutput,
    ) -> Self {
        Self {
            authority: Some(authority),
            processes: Some(PtyProcessCustody::tracking(tree)),
            force: PtyForceTermination::new(),
            output_progress: FaultOutput::Reader(output),
            root_exit_proof: None,
            access: PtyCustodyAccess::Control,
            missing_eof: crate::worker::pty_missing_eof::MissingEofDeadline::production(),
        }
    }

    pub(crate) fn access(&self) -> PtyCustodyAccess {
        self.access
    }

    pub fn enter_observer_only(&mut self) -> bool {
        if self.access.is_observer_only() {
            return false;
        }
        self.access = PtyCustodyAccess::ObserverOnly;
        true
    }

    pub fn is_observer_only(&self) -> bool {
        self.access.is_observer_only()
    }
}

impl PtyOwnership {
    pub(crate) fn custody(&self) -> &PtyOwnershipCustody {
        self.armed_ownership
            .as_ref()
            .expect(OWNERSHIP_CUSTODY_MISSING_DETAIL)
            .custody
            .as_ref()
            .expect(OWNERSHIP_CUSTODY_MISSING_DETAIL)
    }

    pub(crate) fn custody_mut(&mut self) -> &mut PtyOwnershipCustody {
        self.armed_ownership
            .as_mut()
            .expect(OWNERSHIP_CUSTODY_MISSING_DETAIL)
            .custody
            .as_mut()
            .expect(OWNERSHIP_CUSTODY_MISSING_DETAIL)
    }

    pub fn enter_observer_only(&mut self) -> bool {
        self.custody_mut().enter_observer_only()
    }

    pub fn is_observer_only(&self) -> bool {
        self.custody().is_observer_only()
    }
}
