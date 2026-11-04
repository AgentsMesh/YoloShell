use crate::process::{
    CapturedProcessTree, FreezingProcessTree, FrozenProcessTree, KillPendingProcessTree,
    MasterReleasePendingProducersGone, ReportedProducersGone, RootGoneWithoutWait,
    TerminationUnsupportedProcessTree, TrackedProducersGone, UnreapedRootExit,
};
use anyhow::Result;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/integration/pty/pty_process_custody_test_support.rs"]
mod test_support;
#[cfg(test)]
pub(crate) use test_support::{
    PtyProcessCustodyKind, PtyProcessCustodySnapshot, PtyProcessCustodyStateSnapshot,
};

#[path = "pty_process_custody_missing_eof.rs"]
mod missing_eof;

pub(crate) const TRACKING_TREE_UNAVAILABLE_DETAIL: &str =
    "PTY process custody is no longer in the tracking state";

pub(crate) enum PtyProcessCustody {
    Tracking(CapturedProcessTree),
    Freezing(FreezingProcessTree),
    Frozen(FrozenProcessTree),
    Killing(KillPendingProcessTree),
    Survivor(TerminationUnsupportedProcessTree),
    GonePending(TrackedProducersGone),
    GoneReported(ReportedProducersGone),
    MasterClosePending(MasterReleasePendingProducersGone),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct PtyProcessCustodyProgress {
    pub(crate) phase_changed: bool,
    pub(crate) scan_activity: crate::process::CapabilityScanActivity,
}

impl PtyProcessCustody {
    pub(crate) fn tracking(tree: CapturedProcessTree) -> Self {
        Self::Tracking(tree)
    }

    pub(crate) fn tracking_mut(&mut self) -> &mut CapturedProcessTree {
        match self {
            Self::Tracking(tree) => tree,
            _ => panic!("{TRACKING_TREE_UNAVAILABLE_DETAIL}"),
        }
    }

    pub(crate) fn requires_exact_root_force_fallback(&self) -> bool {
        match self {
            Self::Tracking(tree) => tree.require_force_transaction_start().is_err(),
            Self::Freezing(tree) => tree.requires_exact_root_force_fallback(),
            _ => false,
        }
    }

    pub(crate) fn try_tracking_mut(&mut self) -> Option<&mut CapturedProcessTree> {
        match self {
            Self::Tracking(tree) => Some(tree),
            _ => None,
        }
    }

    pub(crate) fn mark_root_exited_unreaped(&mut self, proof: UnreapedRootExit) -> Result<()> {
        match self {
            Self::Tracking(tree) => tree.mark_root_exited_unreaped(proof),
            Self::Freezing(tree) => tree.mark_root_exited_unreaped(proof),
            Self::Frozen(tree) => tree.mark_root_exited_unreaped(proof),
            Self::Killing(tree) => tree.mark_root_exited_unreaped(proof),
            Self::Survivor(tree) => tree.mark_root_exited_unreaped(proof),
            Self::GonePending(completion) => completion.mark_root_exited_unreaped(proof),
            Self::GoneReported(completion) => completion.mark_root_exited_unreaped(proof),
            Self::MasterClosePending(completion) => completion.mark_root_exited_unreaped(proof),
        }
    }

    pub(crate) fn mark_root_gone_without_wait(&mut self, proof: RootGoneWithoutWait) -> Result<()> {
        match self {
            Self::Tracking(tree) => tree.mark_root_gone_without_wait(proof),
            Self::Freezing(tree) => tree.mark_root_gone_without_wait(proof),
            Self::Frozen(tree) => tree.mark_root_gone_without_wait(proof),
            Self::Killing(tree) => tree.mark_root_gone_without_wait(proof),
            Self::Survivor(tree) => tree.mark_root_gone_without_wait(proof),
            Self::GonePending(completion) => completion.mark_root_gone_without_wait(proof),
            Self::GoneReported(completion) => completion.mark_root_gone_without_wait(proof),
            Self::MasterClosePending(completion) => completion.mark_root_gone_without_wait(proof),
        }
    }

    pub(crate) fn session_is_retired(&self) -> bool {
        match self {
            Self::Tracking(tree) => tree.session_is_retired(),
            Self::GoneReported(completion) => completion.session_is_retired(),
            Self::Freezing(_)
            | Self::Frozen(_)
            | Self::Killing(_)
            | Self::Survivor(_)
            | Self::GonePending(_)
            | Self::MasterClosePending(_) => false,
        }
    }

    pub(crate) fn phase(&self) -> std::mem::Discriminant<Self> {
        std::mem::discriminant(self)
    }

    pub(crate) fn take_progress_after(
        &mut self,
        previous: Option<std::mem::Discriminant<Self>>,
    ) -> PtyProcessCustodyProgress {
        let phase_changed = Some(self.phase()) != previous;
        let scan_activity = self.take_scan_activity();
        PtyProcessCustodyProgress {
            phase_changed,
            scan_activity,
        }
    }

    pub(crate) fn clear_stale_progress(&mut self) {
        let _ = self.take_scan_activity();
    }

    pub(crate) fn take_scan_activity(&mut self) -> crate::process::CapabilityScanActivity {
        match self {
            Self::Tracking(tree) => tree.take_capability_scan_activity(),
            Self::Freezing(tree) => tree.take_capability_scan_activity(),
            Self::Frozen(tree) => tree.take_capability_scan_activity(),
            Self::Killing(tree) => tree.take_capability_scan_activity(),
            Self::Survivor(tree) => tree.take_capability_scan_activity(),
            Self::GonePending(_) | Self::GoneReported(_) | Self::MasterClosePending(_) => {
                Default::default()
            }
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/process/pty_process_custody_progress_tests.rs"]
mod progress_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/process/pty_process_custody_root_gone.rs"]
mod root_gone_tests;
