use super::coverage::ReleasedSweepCoverage;
use super::holder_hints::ReleasedHolderHints;
use super::holder_observation::ReleasedHolderQueryObservation;
use super::universe::ReleasedCandidate;
use crate::process::LiveProcessIdentity;
use crate::process::inventory_stable::StableInventory;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Default)]
pub(in crate::process::pty_session_proof) struct ReleasedScanState {
    // Exact membership authority survives descriptor custody changes until process exit.
    pub(super) matched: HashMap<LiveProcessIdentity, ReleasedCandidate>,
    // Coverage is scheduling progress only; it never certifies descriptor absence.
    pub(super) coverage: ReleasedSweepCoverage,
    // Holder hints affect scheduling only and never establish membership authority.
    pub(super) holder_hints: ReleasedHolderHints,
    pub(in crate::process::pty_session_proof) holder_observation: ReleasedHolderQueryObservation,
    pub(super) frozen_inventory: Option<Arc<StableInventory>>,
}

impl ReleasedScanState {
    pub(super) fn excluding(&self, excluded: &HashSet<LiveProcessIdentity>) -> Self {
        let mut next = self.clone();
        next.matched
            .retain(|identity, _candidate| !excluded.contains(identity));
        for identity in excluded {
            next.holder_hints.remove(identity);
        }
        next
    }

    #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
    pub(in crate::process::pty_session_proof) fn adversarial_snapshot(&self) -> String {
        let mut matched = self
            .matched
            .keys()
            .map(LiveProcessIdentity::pid)
            .collect::<Vec<_>>();
        matched.sort_unstable();
        format!(
            "matched={matched:?} holders={:?} {} diagnostic=[{}]",
            self.holder_observation,
            self.coverage.adversarial_snapshot(),
            super::super::macos_scan_diagnostic::snapshot(),
        )
    }

    pub(in crate::process::pty_session_proof) fn restart_preserving_matches(&self) -> Self {
        Self {
            matched: self.matched.clone(),
            holder_hints: self.holder_hints.clone(),
            ..Self::default()
        }
    }

    pub(in crate::process::pty_session_proof) fn frozen_inventory(
        &self,
    ) -> Option<Arc<StableInventory>> {
        self.frozen_inventory.as_ref().map(Arc::clone)
    }

    pub(in crate::process::pty_session_proof) fn clear_frozen_inventory(&mut self) {
        self.frozen_inventory = None;
    }

    pub(in crate::process::pty_session_proof) fn path_is_missing(&self) -> bool {
        self.holder_observation.path_is_missing()
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(in crate::process::pty_session_proof) fn mark_path_missing_for_test(&mut self) {
        self.holder_observation = ReleasedHolderQueryObservation::missing_path();
    }

    pub(in crate::process::pty_session_proof) fn retain_frozen_inventory(
        &mut self,
        inventory: Arc<StableInventory>,
    ) {
        self.frozen_inventory = self.coverage.has_active_frozen_round().then_some(inventory);
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(in crate::process::pty_session_proof) fn install_frozen_inventory_for_test(
        &mut self,
        inventory: Arc<StableInventory>,
    ) {
        self.frozen_inventory = Some(inventory);
    }

    pub(in crate::process::pty_session_proof) fn covers_exact_closure(
        &self,
        closure: &[LiveProcessIdentity],
    ) -> bool {
        !closure.is_empty()
            && closure
                .iter()
                .all(|identity| self.matched.contains_key(identity))
    }

    pub(super) fn matches_all(&self, expected: &HashSet<LiveProcessIdentity>) -> bool {
        expected
            .iter()
            .all(|identity| self.matched.contains_key(identity))
    }

    pub(super) fn reconcile_current(&self, current: &[ReleasedCandidate]) -> Self {
        let mut next = self.clone();
        let current_identities = current
            .iter()
            .map(|candidate| candidate.live)
            .collect::<HashSet<_>>();
        for candidate in current.iter().copied() {
            if let Some(retained) = next.matched.get_mut(&candidate.live) {
                retained.stable |= candidate.stable;
            }
        }
        next.matched.retain(|identity, _candidate| {
            current_identities.contains(identity) || identity.try_matches().unwrap_or(true)
        });
        next
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(in crate::process::pty_session_proof) fn matched_lineages_for_test(&self) -> Vec<u64> {
        let mut lineages = self
            .matched
            .keys()
            .map(LiveProcessIdentity::lineage_id)
            .collect::<Vec<_>>();
        lineages.sort_unstable();
        lineages
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(in crate::process::pty_session_proof) fn pending_lineages_for_test(&self) -> Vec<u64> {
        let (_round_size, pending) = self.coverage.snapshot_for_test();
        pending
            .iter()
            .map(LiveProcessIdentity::lineage_id)
            .collect()
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(in crate::process::pty_session_proof) fn round_size_for_test(&self) -> usize {
        let (round_size, _pending) = self.coverage.snapshot_for_test();
        round_size
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(in crate::process::pty_session_proof) fn queued_entries_for_test(&self) -> usize {
        self.coverage.queued_entries_for_test()
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(super) fn holder_hints_for_test(&self) -> HashSet<LiveProcessIdentity> {
        self.holder_hints.identities().clone()
    }
}
