use super::captured_lineage::{CapturedLineage, LineageEdge, ParentCandidates};
use super::inventory_entry::InventoryEntry;
use super::inventory_stable::StableInventory;
use std::collections::HashSet;

pub(super) fn trusted_parents(
    possible: &ParentCandidates,
    stable: &HashSet<u64>,
) -> ParentCandidates {
    possible
        .iter()
        .filter(|(child, parents)| {
            // Linux parent links are numeric PIDs, not immutable kernel lineage IDs.
            #[cfg(target_os = "linux")]
            let identities_stable =
                stable.contains(child) && parents.iter().all(|parent| stable.contains(parent));
            #[cfg(not(target_os = "linux"))]
            let identities_stable = {
                let _ = (child, stable);
                true
            };
            parents.len() == 1 && identities_stable
        })
        .map(|(child, parents)| (*child, parents.clone()))
        .collect()
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/inventory_parent_trust_tests.rs"]
mod tests;

impl StableInventory {
    pub(super) fn trusted_descendants(
        &self,
        anchors: &HashSet<u64>,
        lineage: &CapturedLineage,
    ) -> HashSet<u64> {
        lineage.descendants(anchors, &self.trusted_parents)
    }

    pub(super) fn stable_session_lineages(&self, session_id: Option<u32>) -> HashSet<u64> {
        self.entries
            .iter()
            .filter(|entry| session_id == Some(entry.session_id))
            .map(|entry| entry.lineage_id())
            .collect()
    }

    pub(super) fn possible_session_lineages(&self, session_id: Option<u32>) -> HashSet<u64> {
        self.observed_sessions
            .iter()
            .filter(|(_, sessions)| session_id.is_some_and(|id| sessions.contains(&id)))
            .map(|(lineage, _)| *lineage)
            .collect()
    }

    pub(super) fn observed_within(&self, owned: &HashSet<u64>) -> HashSet<u64> {
        self.observed_lineages
            .intersection(owned)
            .copied()
            .collect()
    }

    pub(super) fn unstable_within(&self, owned: &HashSet<u64>) -> HashSet<u64> {
        self.unstable_lineages
            .intersection(owned)
            .copied()
            .collect()
    }

    pub(super) fn stable_entries_within(&self, owned: &HashSet<u64>) -> Vec<InventoryEntry> {
        self.entries
            .iter()
            .copied()
            .filter(|entry| owned.contains(&entry.lineage_id()))
            .collect()
    }

    pub(super) fn trusted_edges_within(&self, owned: &HashSet<u64>) -> Vec<LineageEdge> {
        self.trusted_parents
            .iter()
            .flat_map(|(child, parents)| {
                parents.iter().filter_map(|parent| {
                    (owned.contains(child) && owned.contains(parent)).then_some(LineageEdge {
                        child: *child,
                        parent: *parent,
                    })
                })
            })
            .collect()
    }
}
