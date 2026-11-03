use super::CapturedProcessTree;
use super::discovery::DiscoveryEpochProgress;
use super::frozen_discovery::FrozenForkCoverage;
use crate::process::captured_lineage::{LineageObservationStatus, LineageRetention};
use anyhow::Result;
use std::collections::HashSet;

pub(super) const INITIAL_ROOT_MISSING_DETAIL: &str =
    "initial captured process tree does not contain its root";

impl CapturedProcessTree {
    pub(super) fn compact(
        &mut self,
        active_lineages: &HashSet<u64>,
        unsettled_anchors: &HashSet<u64>,
        lineage_status: LineageObservationStatus,
        capability_pending: bool,
        capability_coverage_complete: bool,
        authoritative_empty: bool,
        observed_edges: &[crate::process::captured_lineage::LineageEdge],
    ) -> Result<DiscoveryEpochProgress> {
        self.retiring.retain(|identity| {
            !self.boundary.contains(identity) && unsettled_anchors.contains(&identity.lineage_id())
        });
        let matches = self
            .captured
            .keys()
            .cloned()
            .map(|identity| identity.try_matches().map(|matches| (identity, matches)))
            .collect::<Result<Vec<_>>>()?;
        let mut retired = HashSet::new();
        for (identity, matches) in matches {
            if !matches {
                retired.insert(identity);
            }
        }
        #[cfg(target_os = "linux")]
        if self.root_exit.is_gone() {
            let root = self.scope.live_root();
            retired.extend(
                self.captured
                    .keys()
                    .filter(|identity| {
                        *identity != &root && !active_lineages.contains(&identity.lineage_id())
                    })
                    .cloned(),
            );
        }
        self.captured
            .retain(|identity, _durable| !retired.contains(identity));
        #[cfg(target_os = "macos")]
        self.retiring.extend(
            retired
                .into_iter()
                .filter(|identity| !self.boundary.contains(identity)),
        );
        #[cfg(not(target_os = "macos"))]
        let mut retired_descendant = false;
        #[cfg(not(target_os = "macos"))]
        for identity in retired {
            if identity == self.scope.live_root() {
                self.retiring.insert(identity);
            } else {
                retired_descendant = true;
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.lineage_integrity_lost =
                retired_descendant || lineage_status == LineageObservationStatus::Indeterminate;
        }
        self.lineage_settled = lineage_status == LineageObservationStatus::Complete;
        self.capability_scan_pending = capability_pending;
        let anchors = self
            .captured
            .keys()
            .chain(&self.retiring)
            .filter(|identity| !self.boundary.contains(identity))
            .map(|identity| identity.lineage_id())
            .collect();
        self.lineage.retain_paths(LineageRetention {
            active: active_lineages,
            anchors: &anchors,
        });
        let mut publication_quiescent = self
            .events
            .reconcile_inventory(self.captured.keys().cloned());
        if self.acknowledge_exact_lineage_containment(lineage_status, publication_quiescent) {
            publication_quiescent = false;
        }
        let coverage = FrozenForkCoverage {
            lineage_complete: lineage_status == LineageObservationStatus::Complete,
            capability_coverage_complete,
            publication_quiescent,
        };
        #[cfg(any(test, feature = "runtime-component-tests"))]
        let coverage = self
            .test_state
            .take_frozen_fork_coverage_override()
            .unwrap_or(coverage);
        #[cfg(any(test, feature = "runtime-component-tests"))]
        self.test_state.observe_coverage(coverage);
        let event_uncertain = self.events.lineage_uncertain() || !coverage.publication_quiescent;
        #[cfg(target_os = "macos")]
        {
            self.lineage_integrity_lost = event_uncertain;
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.lineage_integrity_lost |= event_uncertain;
        }
        let lineage_status = if coverage.lineage_complete {
            LineageObservationStatus::Complete
        } else {
            LineageObservationStatus::Indeterminate
        };
        let progress = self.settle_discovery_epoch(
            active_lineages,
            unsettled_anchors,
            lineage_status,
            capability_pending,
            coverage.capability_coverage_complete,
            authoritative_empty,
            observed_edges,
            coverage.publication_quiescent,
        );
        self.observe_frozen_fork_reconciliation(coverage);
        Ok(progress)
    }
}
