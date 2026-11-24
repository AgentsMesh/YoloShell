use super::{FaultReap, PtyProcessCustody, RootWaitAuthority};
use crate::process::CapabilityScanActivity;

use super::schedule::FaultReapActivity;

#[derive(Eq, PartialEq)]
pub(crate) struct FaultReapProgress {
    pub(crate) phase: Option<std::mem::Discriminant<PtyProcessCustody>>,
    pub(crate) root_exit_observed: bool,
    pub(crate) root_gone_without_wait: bool,
    pub(crate) wait_authority_lost: bool,
    pub(crate) root_waited: bool,
}

impl FaultReap {
    pub(crate) fn progress(&self) -> FaultReapProgress {
        FaultReapProgress {
            phase: self
                .custody()
                .processes
                .as_ref()
                .map(PtyProcessCustody::phase),
            root_exit_observed: self.custody().root_exit_proof.is_some(),
            root_gone_without_wait: self.root_gone_without_wait,
            wait_authority_lost: self.lost_wait_custody.is_some(),
            root_waited: self
                .custody()
                .authority
                .as_ref()
                .is_some_and(RootWaitAuthority::is_waited),
        }
    }

    pub(crate) fn take_progress_after(&mut self, previous: FaultReapProgress) -> FaultReapActivity {
        let state_progressed = self.progress() != previous;
        let process_progress = self
            .custody_mut()
            .processes
            .as_mut()
            .map(|custody| custody.take_progress_after(previous.phase));
        if state_progressed
            || process_progress
                .as_ref()
                .is_some_and(|progress| progress.phase_changed)
            || process_progress
                .as_ref()
                .is_some_and(|progress| progress.scan_activity == CapabilityScanActivity::Evidence)
        {
            return observed_activity(FaultReapActivity::SemanticProgress);
        }
        let activity = if process_progress
            .is_some_and(|progress| progress.scan_activity == CapabilityScanActivity::ContinueRound)
        {
            FaultReapActivity::ScanContinuation
        } else {
            FaultReapActivity::Idle
        };
        observed_activity(activity)
    }
}

pub(crate) fn observed_activity(activity: FaultReapActivity) -> FaultReapActivity {
    #[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
    super::adversarial_diagnostic::record_activity(activity);
    activity
}
