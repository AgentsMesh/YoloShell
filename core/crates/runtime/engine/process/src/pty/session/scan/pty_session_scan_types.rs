use super::{ObservedProcess, PtyCapabilityScanState};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::process) enum CapabilityScanStatus {
    Complete,
    Pending,
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum CapabilityScanActivity {
    #[default]
    Idle,
    ContinueRound,
    Evidence,
}

impl CapabilityScanActivity {
    pub(in crate::process) fn merge(self, observed: Self) -> Self {
        std::cmp::max(self, observed)
    }
}

pub(in crate::process) struct CapabilityCandidates {
    pub(in crate::process) stable: HashSet<u64>,
    pub(in crate::process) observed: HashSet<u64>,
    pub(in crate::process) status: CapabilityScanStatus,
    pub(in crate::process) identities: Vec<ObservedProcess>,
}

impl Default for CapabilityCandidates {
    fn default() -> Self {
        Self {
            stable: HashSet::new(),
            observed: HashSet::new(),
            status: CapabilityScanStatus::Complete,
            identities: Vec::new(),
        }
    }
}

pub(in crate::process) struct CapabilityScanAdvance {
    pub(in crate::process) next: PtyCapabilityScanState,
    pub(in crate::process) batch_consumed: bool,
    pub(in crate::process) evidence_observed: bool,
}

pub(in crate::process) struct CapabilityScanResult {
    pub(in crate::process) candidates: CapabilityCandidates,
    pub(in crate::process) advance: CapabilityScanAdvance,
    // Deferred candidates count as scheduled, but never as conclusive coverage.
    pub(in crate::process) scheduling_status: CapabilityScanStatus,
    pub(in crate::process) coverage_status: CapabilityScanStatus,
}

impl CapabilityScanResult {
    pub(in crate::process) fn deferred_for_kernel_eof(state: &PtyCapabilityScanState) -> Self {
        let mut candidates = CapabilityCandidates::default();
        candidates.status = CapabilityScanStatus::Pending;
        Self {
            candidates,
            advance: CapabilityScanAdvance {
                next: state.clone(),
                batch_consumed: false,
                evidence_observed: false,
            },
            scheduling_status: CapabilityScanStatus::Pending,
            coverage_status: CapabilityScanStatus::Pending,
        }
    }
}

impl CapabilityScanAdvance {
    pub(in crate::process) fn activity(
        &self,
        scheduling_status: CapabilityScanStatus,
    ) -> CapabilityScanActivity {
        if self.evidence_observed {
            CapabilityScanActivity::Evidence
        } else if self.batch_consumed && scheduling_status == CapabilityScanStatus::Pending {
            CapabilityScanActivity::ContinueRound
        } else {
            CapabilityScanActivity::Idle
        }
    }
}
