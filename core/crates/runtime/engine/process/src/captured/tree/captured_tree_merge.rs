use super::CapturedProcessTree;
use crate::process::captured_lineage::InventoryObservation;
use anyhow::{Result, ensure};

const DURABLE_IDENTITY_CHANGED_DETAIL: &str =
    "live process identity changed its durable start time";

impl CapturedProcessTree {
    pub(super) fn merge(&mut self, observation: InventoryObservation) -> Result<()> {
        let InventoryObservation {
            mut identities,
            mut edges,
            mut exact_edges,
            mut active_lineages,
            mut unsettled_anchors,
            lineage_status,
            capability_pending,
            capability_scheduling_status,
            capability_coverage_complete,
            capability_advance,
        } = observation;
        let blocked = self.boundary.lineages();
        identities.retain(|process| !self.boundary.contains(&process.live));
        edges.retain(|edge| !blocked.contains(&edge.child) && !blocked.contains(&edge.parent));
        exact_edges
            .retain(|edge| !blocked.contains(&edge.child) && !blocked.contains(&edge.parent));
        active_lineages.retain(|lineage| !blocked.contains(lineage));
        unsettled_anchors.retain(|lineage| !blocked.contains(lineage));
        let authoritative_empty = identities.is_empty() && active_lineages.is_empty();
        for process in &identities {
            if let Some(durable) = self.captured.get(&process.live) {
                ensure!(*durable == process.durable, DURABLE_IDENTITY_CHANGED_DETAIL);
            }
        }
        self.lineage.extend(edges.iter().copied());
        self.captured.extend(
            identities
                .into_iter()
                .map(|process| (process.live, process.durable)),
        );
        let discovery_progress = self.compact(
            &active_lineages,
            &unsettled_anchors,
            lineage_status,
            capability_pending,
            capability_coverage_complete,
            authoritative_empty,
            &edges,
        )?;
        if self.discovery_epoch.observe(discovery_progress) {
            self.discovery_authoritative_empty = discovery_progress.authoritative_empty();
        }
        self.exact_containment_lineage.observe(
            &exact_edges,
            &active_lineages,
            &self.events.unresolved_fork_versions(),
            self.events.inventory_publication_is_current()
                && lineage_status
                    == crate::process::captured_lineage::LineageObservationStatus::Complete,
        );
        let scan_activity = capability_advance.activity(capability_scheduling_status);
        let scheduling_complete = capability_scheduling_status
            == crate::process::pty_session_proof::CapabilityScanStatus::Complete;
        self.capability_scheduling_complete = capability_advance.merge_scheduling_completion(
            &self.pty_scan,
            self.capability_scheduling_complete,
            scheduling_complete,
        );
        self.capability_scan_activity = self.capability_scan_activity.merge(scan_activity);
        self.pty_scan.commit(capability_advance);
        Ok(())
    }
}
