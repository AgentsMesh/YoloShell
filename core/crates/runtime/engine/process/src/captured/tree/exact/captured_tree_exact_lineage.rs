use crate::process::captured_lineage::LineageEdge;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Default)]
pub(super) struct ExactContainmentLineage {
    published_lineages: Option<HashSet<u64>>,
    observed_lineages: HashSet<u64>,
    sources: HashMap<u64, SourceLineage>,
}

struct SourceLineage {
    version: u64,
    baseline: HashSet<u64>,
    children: HashMap<u64, HashSet<u64>>,
}

impl ExactContainmentLineage {
    pub(super) fn observe(
        &mut self,
        edges: &[LineageEdge],
        active: &HashSet<u64>,
        versions: &HashMap<u64, u64>,
        publication_quiescent: bool,
    ) {
        let observed = inventory_lineages(active, edges);
        self.sources
            .retain(|source, _| versions.contains_key(source));
        if let Some(baseline) = self.published_lineages.as_ref() {
            let baseline = baseline
                .union(&self.observed_lineages)
                .copied()
                .collect::<HashSet<_>>();
            for (&source, &version) in versions {
                match self.sources.get_mut(&source) {
                    Some(state) if state.version == version => {}
                    Some(state) => {
                        let mut transitioned = baseline.clone();
                        transitioned.extend(&observed);
                        *state = SourceLineage::new(version, transitioned);
                    }
                    None => {
                        self.sources
                            .insert(source, SourceLineage::new(version, baseline.clone()));
                    }
                }
            }
        }
        self.observed_lineages.extend(&observed);
        if !publication_quiescent {
            return;
        }
        for state in self.sources.values_mut() {
            state.extend(edges);
        }
        self.observed_lineages = observed.clone();
        self.published_lineages = Some(observed);
    }

    pub(super) fn related_sources(
        &self,
        versions: &HashMap<u64, u64>,
        targets: &HashSet<u64>,
    ) -> HashMap<u64, u64> {
        versions
            .iter()
            .filter_map(|(&source, &version)| {
                self.sources
                    .get(&source)
                    .filter(|state| {
                        state.version == version && state.reaches_target(source, targets)
                    })
                    .map(|_| (source, version))
            })
            .collect()
    }

    pub(super) fn clear(&mut self) {
        self.sources.clear();
    }
}

fn inventory_lineages(active: &HashSet<u64>, edges: &[LineageEdge]) -> HashSet<u64> {
    let mut observed = active.clone();
    for edge in edges {
        observed.extend([edge.parent, edge.child]);
    }
    observed
}

impl SourceLineage {
    fn new(version: u64, baseline: HashSet<u64>) -> Self {
        Self {
            version,
            baseline,
            children: HashMap::new(),
        }
    }

    fn extend(&mut self, edges: &[LineageEdge]) {
        for edge in edges {
            self.children
                .entry(edge.parent)
                .or_default()
                .insert(edge.child);
        }
    }

    fn reaches_target(&self, source: u64, targets: &HashSet<u64>) -> bool {
        let fresh_children = self
            .children
            .get(&source)
            .into_iter()
            .flatten()
            .filter(|child| !self.baseline.contains(child))
            .copied();
        let mut pending = VecDeque::from_iter(fresh_children);
        let mut visited = HashSet::from([source]);
        while let Some(lineage) = pending.pop_front() {
            if !visited.insert(lineage) {
                continue;
            }
            if targets.contains(&lineage) {
                return true;
            }
            pending.extend(self.children.get(&lineage).into_iter().flatten().copied());
        }
        false
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/captured/tree/exact/captured_tree_exact_lineage_tests.rs"]
mod tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/captured/tree/exact/captured_tree_exact_lineage_version_tests.rs"]
mod version_tests;
