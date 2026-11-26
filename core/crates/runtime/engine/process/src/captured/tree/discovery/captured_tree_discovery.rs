use super::CapturedProcessTree;
use crate::process::captured_lineage::{LineageEdge, LineageObservationStatus};
use crate::process::{DescendantObservation, ProcessIdentity, PtySessionProof};
use anyhow::Result;
use std::collections::HashSet;

#[path = "captured_tree_discovery_epoch.rs"]
mod epoch;
pub(super) use epoch::{DiscoveryEpochProgress, DiscoveryScanEpoch};

impl CapturedProcessTree {
    pub(crate) fn capture_with_discovery_internal(
        root: ProcessIdentity,
        capability: String,
    ) -> Result<Self> {
        let proof = PtySessionProof::discovery(capability)?;
        Self::capture_pty(root, proof)
    }

    pub(crate) fn begin_discovery_cleanup_internal(&mut self) {
        #[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
        crate::process::pty_session_proof::macos_scan_diagnostic::record_discovery_restart();
        self.reset_discovery_publication_audit();
        self.discovery_epoch = DiscoveryScanEpoch::Pending;
        self.discovery_authoritative_empty = false;
        self.capability_scan_pending = true;
        self.capability_scheduling_complete = false;
        self.pty_scan.restart();
    }

    pub(super) fn begin_root_exit_discovery_epoch(&mut self) {
        #[cfg(target_os = "macos")]
        if self.pty_proof.is_some() {
            self.begin_discovery_cleanup_internal();
        }
    }

    pub(super) fn restart_discovery_after_event(&mut self, invalidated: bool) {
        #[cfg(target_os = "macos")]
        if invalidated {
            self.capability_scheduling_complete = false;
            self.pty_scan.invalidate_frozen_inventory();
        }
        #[cfg(target_os = "macos")]
        if invalidated
            && self.root_exit_is_proven()
            && self.pty_proof.is_some()
            && self.discovery_epoch == DiscoveryScanEpoch::Complete
        {
            self.begin_discovery_cleanup_internal();
        }
        #[cfg(not(target_os = "macos"))]
        let _ = invalidated;
    }

    pub(crate) fn discovery_is_settled_internal(&self) -> bool {
        self.discovery_epoch == DiscoveryScanEpoch::Complete
            && self.descendant_observation() == DescendantObservation::Settled
    }

    #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
    pub(crate) fn discovery_coverage_is_complete_internal(&self) -> bool {
        self.discovery_epoch.coverage_is_complete()
    }

    pub(crate) fn discovery_is_authoritatively_empty_internal(&self) -> bool {
        self.discovery_is_settled_internal() && self.discovery_authoritative_empty
    }

    pub(super) fn root_exit_is_proven(&self) -> bool {
        self.root_exit.is_gone() || self.pty_scan.proves_root_exit()
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn root_exit_is_proven_for_test(&self) -> bool {
        self.root_exit_is_proven()
    }

    pub(crate) fn refresh_if_needed_internal(&mut self) -> Result<bool> {
        self.refresh_if_changed()
    }

    pub(super) fn settle_discovery_epoch(
        &mut self,
        active_lineages: &HashSet<u64>,
        unsettled_anchors: &HashSet<u64>,
        lineage_status: LineageObservationStatus,
        capability_pending: bool,
        capability_coverage_complete: bool,
        authoritative_empty: bool,
        observed_edges: &[LineageEdge],
        publication_quiescent: bool,
    ) -> DiscoveryEpochProgress {
        #[cfg(target_os = "macos")]
        {
            #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
            crate::process::pty_session_proof::macos_scan_diagnostic::record_discovery_gap(
                capability_coverage_complete,
                lineage_status == LineageObservationStatus::Complete,
                publication_quiescent,
            );
            if self.pty_proof.is_none()
                || !self.discovery_epoch.requested()
                || !capability_coverage_complete
                || lineage_status != LineageObservationStatus::Complete
                || !publication_quiescent
            {
                return DiscoveryEpochProgress::default();
            }
            let unresolved_fork = self.events.has_unresolved_fork();
            if unresolved_fork && (!self.root_exit_is_proven() || capability_pending) {
                #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
                crate::process::pty_session_proof::macos_scan_diagnostic::record_discovery_coverage(
                    false,
                );
                return DiscoveryEpochProgress::coverage_only(authoritative_empty);
            }
            if self.events.is_gated() {
                #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
                crate::process::pty_session_proof::macos_scan_diagnostic::record_discovery_coverage(
                    true,
                );
                if unresolved_fork {
                    self.events.acknowledge_root_exit_discovery_epoch();
                }
                self.lineage_integrity_lost = false;
                return DiscoveryEpochProgress::covered(capability_pending, authoritative_empty);
            }
            if self.is_stable_empty_fallback(active_lineages, unsettled_anchors, observed_edges) {
                self.lineage_integrity_lost = false;
                return DiscoveryEpochProgress::covered(capability_pending, authoritative_empty);
            }
            DiscoveryEpochProgress::default()
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (
                active_lineages,
                unsettled_anchors,
                lineage_status,
                observed_edges,
                publication_quiescent,
            );
            if capability_coverage_complete {
                DiscoveryEpochProgress::covered(capability_pending, authoritative_empty)
            } else {
                DiscoveryEpochProgress::default()
            }
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/captured/tree/discovery/captured_tree_discovery_state_tests.rs"]
mod state_tests;
