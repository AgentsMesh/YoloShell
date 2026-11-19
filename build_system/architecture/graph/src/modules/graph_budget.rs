const MINIMUM_GRAPH_FACTOR: usize = 1;
const MAXIMUM_GRAPH_STATES: usize = 65_536;

pub(super) fn states(roots: usize, sources: usize, edges: usize) -> usize {
    roots
        .max(MINIMUM_GRAPH_FACTOR)
        .saturating_mul(sources.max(MINIMUM_GRAPH_FACTOR))
        .saturating_mul(edges.max(MINIMUM_GRAPH_FACTOR))
        .min(MAXIMUM_GRAPH_STATES)
}

#[cfg(test)]
#[path = "../../unit_tests/graph_budget_tests.rs"]
mod tests;
