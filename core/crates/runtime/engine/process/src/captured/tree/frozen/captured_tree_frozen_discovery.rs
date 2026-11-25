use super::CapturedProcessTree;
use crate::process::LiveProcessIdentity;
#[cfg(target_os = "macos")]
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FrozenForkCoverage {
    pub(super) lineage_complete: bool,
    pub(super) capability_coverage_complete: bool,
    pub(super) publication_quiescent: bool,
}

impl FrozenForkCoverage {
    fn is_complete(self) -> bool {
        self.lineage_complete && self.capability_coverage_complete && self.publication_quiescent
    }
}

#[cfg(any(test, feature = "runtime-component-tests"))]
impl FrozenForkCoverage {
    pub(super) const MISSING_LINEAGE: Self = Self {
        lineage_complete: false,
        capability_coverage_complete: true,
        publication_quiescent: true,
    };
    pub(super) const MISSING_CAPABILITY: Self = Self {
        lineage_complete: true,
        capability_coverage_complete: false,
        publication_quiescent: true,
    };
    pub(super) const MISSING_PUBLICATION: Self = Self {
        lineage_complete: true,
        capability_coverage_complete: true,
        publication_quiescent: false,
    };
}

impl CapturedProcessTree {
    pub(super) fn begin_frozen_discovery(&mut self) {
        #[cfg(target_os = "macos")]
        if self.pty_proof.is_some() {
            #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
            crate::process::pty_session_proof::macos_scan_diagnostic::record_frozen_begin();
            self.begin_discovery_cleanup_internal();
        }
    }

    pub(super) fn defer_frozen_discovery_for_event(&mut self) {
        #[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
        crate::process::pty_session_proof::macos_scan_diagnostic::record_frozen_event_deferral();
        self.begin_frozen_discovery();
    }

    pub(super) fn acknowledge_frozen_forks(
        &mut self,
        frozen_sources: &[LiveProcessIdentity],
    ) -> bool {
        #[cfg(target_os = "macos")]
        {
            let retained_bootstrap_coverage = self
                .pty_proof
                .as_ref()
                .is_some_and(super::PtySessionProof::bootstrap_capability_coverage_is_retained);
            let positive_closure_coverage =
                self.frozen_fork_acknowledgement_coverage(frozen_sources);
            if (!self.discovery_epoch.coverage_is_complete()
                && !retained_bootstrap_coverage
                && !positive_closure_coverage)
                || !self.lineage_settled
                || (self.capability_scan_pending && !positive_closure_coverage)
            {
                return false;
            }
            let frozen_lineages = frozen_sources
                .iter()
                .map(LiveProcessIdentity::lineage_id)
                .collect::<HashSet<_>>();
            let acknowledged = self.events.acknowledge_frozen_forks(&frozen_lineages);
            let retained_fork_requires_rescan =
                retained_bootstrap_coverage && self.events.has_unresolved_fork();
            if acknowledged || retained_fork_requires_rescan {
                self.frozen_fork_reconciliation_required = true;
                if let Some(proof) = self.pty_proof.as_mut() {
                    proof.clear_bootstrap_capability_coverage();
                }
                self.begin_discovery_cleanup_internal();
                return true;
            }
            if !self.events.has_unresolved_fork() && self.bootstrap_exec_fence_coverage() {
                self.capability_scan_pending = false;
                if self.discovery_epoch.coverage_is_complete()
                    && let Some(proof) = self.pty_proof.as_mut()
                {
                    proof.retain_bootstrap_capability_coverage();
                }
            }
            false
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = frozen_sources;
            false
        }
    }

    pub(super) fn observe_frozen_fork_reconciliation(&mut self, coverage: FrozenForkCoverage) {
        if self.frozen_fork_reconciliation_required && coverage.is_complete() {
            self.frozen_fork_reconciliation_required = false;
        }
    }

    #[cfg(all(test, target_os = "macos"))]
    pub(crate) fn inject_unresolved_root_fork_for_test(&mut self) {
        self.events.inject_unresolved_fork(self.live_root());
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../../../unit_tests/captured_tree_frozen_discovery_inline_tests.rs"]
mod tests;
