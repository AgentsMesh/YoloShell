use super::super::captured_lineage::{CapturedLineage, LineageEdge, ParentCandidates};
use super::super::inventory::SessionOwnershipProof;
use super::super::inventory_stable::StableInventory;
use crate::process::LiveProcessIdentity;
use std::collections::HashSet;

pub(super) fn lineage_anchors(
    requested: &HashSet<LiveProcessIdentity>,
    _live: &[LiveProcessIdentity],
    ownership: SessionOwnershipProof,
) -> HashSet<u64> {
    #[cfg(target_os = "macos")]
    let identities = requested.iter();
    #[cfg(not(target_os = "macos"))]
    let identities = _live.iter();
    #[cfg(target_os = "linux")]
    if ownership.root_is_gone() {
        return HashSet::new();
    }
    #[cfg(not(target_os = "linux"))]
    let _ = ownership;
    identities.map(LiveProcessIdentity::lineage_id).collect()
}

pub(super) fn joined(left: &HashSet<u64>, right: &HashSet<u64>) -> HashSet<u64> {
    left.union(right).copied().collect()
}

pub(super) fn closure(
    seeds: &HashSet<u64>,
    lineage: &CapturedLineage,
    parents: &ParentCandidates,
) -> HashSet<u64> {
    seeds
        .union(&lineage.descendants(seeds, parents))
        .copied()
        .collect()
}

pub(super) fn parents_with_membership(
    parents: &ParentCandidates,
    members: &HashSet<u64>,
    root: u64,
    blocked: &HashSet<u64>,
) -> ParentCandidates {
    let mut combined: ParentCandidates = parents
        .iter()
        .filter(|(child, _parents)| !blocked.contains(child))
        .map(|(child, parents)| {
            (
                *child,
                parents
                    .iter()
                    .filter(|parent| !blocked.contains(parent))
                    .copied()
                    .collect(),
            )
        })
        .collect();
    for member in members.iter().copied().filter(|member| *member != root) {
        combined.entry(member).or_default().insert(root);
    }
    combined
}

pub(super) fn membership_edges(members: &HashSet<u64>, root: u64) -> Vec<LineageEdge> {
    members
        .iter()
        .copied()
        .filter(|member| *member != root)
        .map(|child| LineageEdge {
            child,
            parent: root,
        })
        .collect()
}

pub(super) fn unsettled_anchors(
    inventory: &StableInventory,
    possible_owned: &HashSet<u64>,
    anchors: &HashSet<u64>,
    lineage: &CapturedLineage,
    parents: &ParentCandidates,
) -> HashSet<u64> {
    inventory
        .unstable_within(possible_owned)
        .into_iter()
        .flat_map(|unstable| lineage.nearest_anchors(unstable, anchors, parents))
        .collect()
}
