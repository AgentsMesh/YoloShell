use super::captured_lineage::{
    CapturedLineage, InventoryObservation, LineageObservationStatus, ObservedProcess,
};
use super::controller_boundary::ProcessDiscoveryBoundary;
use super::inventory::{PtyProcessScope, SessionOwnershipProof};
use super::pty_session_proof::{CapabilityScanStatus, PtyCapabilityScanState};
use super::{LiveProcessIdentity, PtySessionProof};
use anyhow::Result;
use std::collections::HashSet;
#[path = "inventory_observation_cache.rs"]
pub(in crate::process) mod cache;
#[path = "inventory_observation_graph.rs"]
mod graph;
use graph::{
    joined, lineage_anchors, membership_edges, parents_with_membership, unsettled_anchors,
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum CapabilityObservationMode {
    Scan,
    KernelEofDeferred,
}

impl PtyProcessScope {
    pub(super) fn observe_with_proof(
        &self,
        anchors: &HashSet<LiveProcessIdentity>,
        boundary: &ProcessDiscoveryBoundary,
        lineage: &CapturedLineage,
        ownership_proof: SessionOwnershipProof,
        pty_proof: Option<&PtySessionProof>,
        pty_scan: &PtyCapabilityScanState,
        discovery_scan_requested: bool,
        capability_mode: CapabilityObservationMode,
    ) -> Result<InventoryObservation> {
        let inventory = cache::capture(pty_scan)?;
        let blocked = boundary.lineages();
        let root_exited = ownership_proof.root_is_gone();
        let requested = self.requested_anchors(anchors, boundary, root_exited);
        let mut live_roots = Vec::new();
        for identity in &requested {
            if inventory.contains(identity) && identity.try_matches()? {
                live_roots.push(identity.clone());
            }
        }
        let mut lineage_anchors = lineage_anchors(&requested, &live_roots, ownership_proof);
        #[cfg(target_os = "macos")]
        if root_exited {
            lineage_anchors.insert(self.root.lineage_id());
        }
        let live_session = self.live_session_id(&inventory, &live_roots, ownership_proof);
        let capability_proof = (ownership_proof.enables_capability_scan()
            || discovery_scan_requested)
            .then_some(pty_proof)
            .flatten();
        let defer_capability_scan = capability_mode == CapabilityObservationMode::KernelEofDeferred
            && capability_proof.is_some();
        let mut capability = if defer_capability_scan {
            super::pty_session_proof::CapabilityScanResult::deferred_for_kernel_eof(pty_scan)
        } else {
            inventory.capability_candidates(
                capability_proof,
                pty_scan,
                &requested,
                boundary.identities(),
                self.durable_root,
                self.root.lineage_id(),
                root_exited,
                discovery_scan_requested,
            )?
        };
        cache::retain(&mut capability, &inventory);
        let scheduling_status = capability.scheduling_status;
        let coverage_complete = capability.coverage_status == CapabilityScanStatus::Complete;
        let capability_advance = capability.advance;
        let mut capability = capability.candidates;
        capability
            .stable
            .retain(|lineage| !blocked.contains(lineage));
        capability
            .observed
            .retain(|lineage| !blocked.contains(lineage));
        capability
            .identities
            .retain(|process| !boundary.contains(&process.live));
        let direct_lineages = capability
            .identities
            .iter()
            .map(|process| process.live.lineage_id())
            .collect::<HashSet<_>>();
        let mut stable_members = joined(
            &inventory.stable_session_lineages(live_session),
            &capability.stable,
        );
        let mut possible_members = joined(
            &inventory.possible_session_lineages(live_session),
            &capability.observed,
        );
        stable_members.retain(|lineage| !blocked.contains(lineage));
        possible_members.retain(|lineage| !blocked.contains(lineage));
        if root_exited {
            stable_members.remove(&self.root.lineage_id());
            possible_members.remove(&self.root.lineage_id());
        }
        let trusted_parents = parents_with_membership(
            &inventory.trusted_parents,
            &stable_members,
            self.root.lineage_id(),
            &blocked,
        );
        let possible_parents = parents_with_membership(
            &inventory.possible_parents,
            &possible_members,
            self.root.lineage_id(),
            &blocked,
        );
        let stable_seeds = joined(&lineage_anchors, &stable_members);
        let possible_seeds = joined(&lineage_anchors, &possible_members);
        let stable_owned = graph::closure(&stable_seeds, lineage, &trusted_parents);
        let possible_owned = graph::closure(&possible_seeds, lineage, &possible_parents);
        let mut identities = Vec::new();
        let mut lineage_status = LineageObservationStatus::Complete;
        for entry in inventory.stable_entries_within(&stable_owned) {
            if blocked.contains(&entry.lineage_id())
                || direct_lineages.contains(&entry.lineage_id())
            {
                continue;
            }
            if let Some(live) = inventory.live_identity(entry)? {
                identities.push(ObservedProcess {
                    live,
                    durable: entry.identity(),
                });
            } else {
                lineage_status = LineageObservationStatus::Indeterminate;
            }
        }
        identities.extend(capability.identities);
        let exact_edges = inventory
            .trusted_edges_within(&possible_owned)
            .into_iter()
            .filter(|edge| !blocked.contains(&edge.child) && !blocked.contains(&edge.parent))
            .collect::<Vec<_>>();
        let mut edges = exact_edges.clone();
        edges.extend(membership_edges(&possible_members, self.root.lineage_id()));
        let mut active_lineages = inventory.observed_within(&possible_owned);
        active_lineages.retain(|lineage| !blocked.contains(lineage));
        let mut unsettled_anchors = unsettled_anchors(
            &inventory,
            &possible_owned,
            &lineage_anchors,
            lineage,
            &possible_parents,
        );
        if !unsettled_anchors.is_empty() {
            lineage_status = LineageObservationStatus::Indeterminate;
        }
        let capability_pending = capability.status == CapabilityScanStatus::Pending;
        if capability_pending {
            unsettled_anchors.insert(self.root.lineage_id());
        }
        unsettled_anchors.retain(|lineage| !blocked.contains(lineage));
        Ok(InventoryObservation {
            identities,
            edges,
            exact_edges,
            active_lineages,
            unsettled_anchors,
            lineage_status,
            capability_pending,
            capability_scheduling_status: scheduling_status,
            capability_coverage_complete: coverage_complete,
            capability_advance,
        })
    }
    fn requested_anchors(
        &self,
        anchors: &HashSet<LiveProcessIdentity>,
        boundary: &ProcessDiscoveryBoundary,
        root_exited: bool,
    ) -> HashSet<LiveProcessIdentity> {
        let mut requested: HashSet<LiveProcessIdentity> = anchors
            .iter()
            .filter(|identity| {
                !boundary.contains(identity) && (!root_exited || *identity != &self.root)
            })
            .cloned()
            .collect();
        if !root_exited {
            requested.insert(self.root.clone());
        }
        requested
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/inventory/inventory_observation_tests.rs"]
mod tests;
