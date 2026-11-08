use super::pty_session_proof::{CapabilityScanAdvance, CapabilityScanStatus};
use super::{LiveProcessIdentity, ProcessIdentity};
use std::collections::{HashMap, HashSet, VecDeque};

pub(super) type ParentCandidates = HashMap<u64, HashSet<u64>>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct LineageEdge {
    pub(super) child: u64,
    pub(super) parent: u64,
}

#[derive(Default)]
pub(super) struct CapturedLineage {
    edges: HashSet<LineageEdge>,
    children: HashMap<u64, HashSet<u64>>,
    parents: HashMap<u64, HashSet<u64>>,
}

impl CapturedLineage {
    pub(super) fn extend(&mut self, edges: impl IntoIterator<Item = LineageEdge>) {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = edges;
            return;
        }
        #[cfg(target_os = "macos")]
        for edge in edges {
            if self.edges.insert(edge) {
                self.children
                    .entry(edge.parent)
                    .or_default()
                    .insert(edge.child);
                self.parents
                    .entry(edge.child)
                    .or_default()
                    .insert(edge.parent);
            }
        }
    }

    pub(super) fn retain_paths(&mut self, retention: LineageRetention<'_>) {
        let mut pending = VecDeque::from_iter(retention.active.iter().copied());
        let mut visited = retention.active.clone();
        let mut retained = HashSet::new();
        while let Some(child) = pending.pop_front() {
            if retention.anchors.contains(&child) {
                continue;
            }
            for parent in self.parents.get(&child).into_iter().flatten().copied() {
                retained.insert(LineageEdge { child, parent });
                if visited.insert(parent) {
                    pending.push_back(parent);
                }
            }
        }
        self.edges = retained;
        self.rebuild_indexes();
    }

    #[cfg(test)]
    pub(super) fn reaches(
        &self,
        start: u64,
        anchors: &HashSet<u64>,
        current_parents: &ParentCandidates,
    ) -> bool {
        self.descendants(anchors, current_parents).contains(&start)
    }

    pub(super) fn descendants(
        &self,
        anchors: &HashSet<u64>,
        current_parents: &ParentCandidates,
    ) -> HashSet<u64> {
        let mut current_children: HashMap<u64, HashSet<u64>> = HashMap::new();
        for (child, parents) in current_parents {
            for parent in parents {
                current_children.entry(*parent).or_default().insert(*child);
            }
        }
        let mut pending = VecDeque::from_iter(anchors.iter().copied());
        let mut visited = anchors.clone();
        let mut descendants = HashSet::new();
        while let Some(parent) = pending.pop_front() {
            for child in current_children.get(&parent).into_iter().flatten().copied() {
                if visited.insert(child) {
                    descendants.insert(child);
                    pending.push_back(child);
                }
            }
            #[cfg(target_os = "macos")]
            for child in self.children.get(&parent).into_iter().flatten().copied() {
                if visited.insert(child) {
                    descendants.insert(child);
                    pending.push_back(child);
                }
            }
        }
        descendants
    }

    pub(super) fn nearest_anchors(
        &self,
        start: u64,
        anchors: &HashSet<u64>,
        current_parents: &ParentCandidates,
    ) -> HashSet<u64> {
        let mut pending = VecDeque::from([start]);
        let mut visited = HashSet::from([start]);
        let mut found = HashSet::new();
        while let Some(child) = pending.pop_front() {
            if anchors.contains(&child) {
                found.insert(child);
                continue;
            }
            for parent in current_parents.get(&child).into_iter().flatten().copied() {
                if visited.insert(parent) {
                    pending.push_back(parent);
                }
            }
            #[cfg(target_os = "macos")]
            for parent in self.parents.get(&child).into_iter().flatten().copied() {
                if visited.insert(parent) {
                    pending.push_back(parent);
                }
            }
        }
        found
    }

    fn rebuild_indexes(&mut self) {
        self.children.clear();
        self.parents.clear();
        for edge in self.edges.iter().copied() {
            self.children
                .entry(edge.parent)
                .or_default()
                .insert(edge.child);
            self.parents
                .entry(edge.child)
                .or_default()
                .insert(edge.parent);
        }
    }
}

pub(super) struct LineageRetention<'a> {
    pub(super) active: &'a HashSet<u64>,
    pub(super) anchors: &'a HashSet<u64>,
}

pub(super) struct InventoryObservation {
    pub(super) identities: Vec<ObservedProcess>,
    pub(super) edges: Vec<LineageEdge>,
    pub(super) exact_edges: Vec<LineageEdge>,
    pub(super) active_lineages: HashSet<u64>,
    pub(super) unsettled_anchors: HashSet<u64>,
    pub(super) lineage_status: LineageObservationStatus,
    pub(super) capability_pending: bool,
    pub(super) capability_scheduling_status: CapabilityScanStatus,
    pub(super) capability_coverage_complete: bool,
    pub(super) capability_advance: CapabilityScanAdvance,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum LineageObservationStatus {
    #[default]
    Complete,
    Indeterminate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ObservedProcess {
    pub(super) live: LiveProcessIdentity,
    pub(super) durable: ProcessIdentity,
}

#[cfg(not(target_os = "linux"))]
impl Copy for ObservedProcess {}
