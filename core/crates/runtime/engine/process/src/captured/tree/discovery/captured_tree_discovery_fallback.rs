use super::CapturedProcessTree;
use crate::process::captured_lineage::LineageEdge;
use std::collections::HashSet;

impl CapturedProcessTree {
    pub(super) fn is_stable_empty_fallback(
        &self,
        active_lineages: &HashSet<u64>,
        unsettled_anchors: &HashSet<u64>,
        observed_edges: &[LineageEdge],
    ) -> bool {
        let root = self.scope.live_root();
        self.root_exit_is_proven()
            && active_lineages.is_empty()
            && unsettled_anchors.is_empty()
            && observed_edges.is_empty()
            && self.captured.keys().all(|identity| identity == &root)
            && self.retiring.iter().all(|identity| identity == &root)
    }
}
