use super::FaultReap;
use crate::model::{FaultReapProgress, FaultReapProgressKind, FaultReapStatus};
use crate::worker::pty_ownership::PtyCustodyAccess;
#[cfg(test)]
use crate::worker::pty_process_custody::PtyProcessCustodyStateSnapshot;
use std::sync::{Arc, Mutex};

pub(crate) const PROGRESS_SEQUENCE_INCREMENT: u64 = 1;

pub(crate) type SharedState = Arc<Mutex<ObservedState>>;

#[derive(Default)]
pub(crate) struct ObservedState {
    status: Option<FaultReapStatus>,
    pending_fault: Option<String>,
    #[cfg(test)]
    latest: Option<FaultReapStateSnapshot>,
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FaultReapStateSnapshot {
    pub(crate) custody: Option<PtyProcessCustodyStateSnapshot>,
    pub(crate) wait_authority_present: bool,
    pub(crate) wait_authority_lost: bool,
    pub(crate) root_exit_proof_present: bool,
    pub(crate) root_waited: bool,
    pub(crate) root_gone_without_wait: bool,
    pub(crate) observer_only: bool,
    pub(crate) completion_published: bool,
    pub(crate) failure_present: bool,
    pub(crate) output_fence_reached: Option<bool>,
}

#[derive(Clone, Default)]
pub(crate) struct FaultReapStateObserver {
    pub(crate) shared: SharedState,
}

impl FaultReapStateObserver {
    pub(crate) fn status(&self) -> Option<FaultReapStatus> {
        self.lock().status.clone()
    }

    pub(crate) fn shares_state_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.shared, &other.shared)
    }

    pub(crate) fn publish_handoff(
        &self,
        failure: Option<&anyhow::Error>,
        access: PtyCustodyAccess,
    ) {
        let mut state = self.lock();
        if state.status.is_some() {
            return;
        }
        let last_fault = failure
            .map(crate::worker::terminal_reporting::error_ref_text)
            .or_else(|| state.pending_fault.take());
        state.status = Some(FaultReapStatus {
            custody_retained: true,
            observer_only: access.is_observer_only(),
            last_semantic_progress: None,
            last_fault,
        });
        publish_progress(&mut state, FaultReapProgressKind::Handoff);
    }

    pub(crate) fn publish_progress(&self, kind: FaultReapProgressKind) {
        publish_progress(&mut self.lock(), kind);
    }

    pub(crate) fn publish_observer_only(&self) {
        let mut state = self.lock();
        let Some(status) = state.status.as_mut() else {
            return;
        };
        status.observer_only = true;
        publish_progress(&mut state, FaultReapProgressKind::ObserverOnly);
    }

    pub fn publish_fault(&self, error: &anyhow::Error) {
        let detail = crate::worker::terminal_reporting::error_ref_text(error);
        let mut state = self.lock();
        if let Some(status) = state.status.as_mut() {
            status.last_fault = Some(detail);
        } else if state.pending_fault.is_none() {
            state.pending_fault = Some(detail);
        }
    }

    #[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
    pub(crate) fn publish_adversarial_diagnostic(&self, diagnostic: String) {
        if let Some(status) = self.lock().status.as_mut() {
            status.last_fault = Some(diagnostic);
        }
    }

    pub(crate) fn publish_terminal(&self) {
        let mut state = self.lock();
        let Some(status) = state.status.as_mut() else {
            return;
        };
        status.custody_retained = false;
        publish_progress(&mut state, FaultReapProgressKind::Completed);
    }

    pub(crate) fn lock(&self) -> std::sync::MutexGuard<'_, ObservedState> {
        self.shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[cfg(test)]
    pub(crate) fn latest(&self) -> Option<FaultReapStateSnapshot> {
        self.lock().latest.clone()
    }

    #[cfg(test)]
    pub(crate) fn publish_snapshot(&self, snapshot: FaultReapStateSnapshot) {
        self.lock().latest = Some(snapshot);
    }
}

pub(crate) fn publish_progress(state: &mut ObservedState, kind: FaultReapProgressKind) {
    let Some(status) = state.status.as_mut() else {
        return;
    };
    let sequence = status
        .last_semantic_progress
        .as_ref()
        .map(|progress| progress.sequence)
        .unwrap_or_default()
        .saturating_add(PROGRESS_SEQUENCE_INCREMENT);
    status.last_semantic_progress = Some(FaultReapProgress {
        sequence,
        observed_at_ms: crate::model::now_ms(),
        kind,
    });
}

impl FaultReap {
    pub(crate) fn publish_handoff(&self) {
        self.state_observer
            .publish_handoff(self.failure.as_ref(), self.access());
    }

    pub(crate) fn publish_cleanup_progress(&self) {
        self.state_observer
            .publish_progress(FaultReapProgressKind::CleanupAdvanced);
    }

    pub(crate) fn enter_observer_only(&mut self) {
        if self.custody_mut().enter_observer_only() {
            self.state_observer.publish_observer_only();
        }
    }

    pub(crate) fn publish_fault(&self, error: &anyhow::Error) {
        self.state_observer.publish_fault(error);
    }

    pub(crate) fn publish_terminal_status(&self) {
        self.state_observer.publish_terminal();
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
include!("../../../../test_support/unit/pty_fault_reaper/state_observer_test_support.rs");
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../../unit_tests/pty_fault_reaper/state_observer_status_tests.rs"]
mod status_tests;
