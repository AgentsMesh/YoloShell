use super::ExactContainmentLineage;
use crate::process::captured_lineage::LineageEdge;
use std::collections::{HashMap, HashSet};

const SOURCE: u64 = 21;
const INTERMEDIATE: u64 = 22;
const TARGET: u64 = 23;
const FIRST_VERSION: u64 = 1;
const SECOND_VERSION: u64 = 2;

#[test]
fn a_later_version_cannot_reuse_an_unquiescent_prior_child() {
    let mut lineage = ExactContainmentLineage::default();
    publish(&mut lineage);
    observe(&mut lineage, FIRST_VERSION, full_members(), edges(), false);
    observe(&mut lineage, SECOND_VERSION, full_members(), edges(), true);

    assert!(related(&lineage, SECOND_VERSION).is_empty());
}

#[test]
fn a_later_quiescent_inventory_can_confirm_the_same_version_child() {
    let mut lineage = ExactContainmentLineage::default();
    publish(&mut lineage);
    observe(&mut lineage, FIRST_VERSION, full_members(), edges(), false);
    observe(&mut lineage, FIRST_VERSION, full_members(), edges(), true);

    assert_eq!(
        related(&lineage, FIRST_VERSION),
        HashMap::from([(SOURCE, FIRST_VERSION)])
    );
}

#[test]
fn a_transition_snapshot_cannot_assign_a_prior_version_child() {
    let mut lineage = ExactContainmentLineage::default();
    publish(&mut lineage);
    observe(&mut lineage, FIRST_VERSION, &[SOURCE], &[], false);
    observe(&mut lineage, SECOND_VERSION, full_members(), edges(), false);
    observe(&mut lineage, SECOND_VERSION, full_members(), edges(), true);

    assert!(related(&lineage, SECOND_VERSION).is_empty());
}

#[test]
fn a_resolved_source_retires_before_later_edges_are_observed() {
    let mut lineage = ExactContainmentLineage::default();
    publish(&mut lineage);
    observe(&mut lineage, FIRST_VERSION, full_members(), edges(), true);
    assert!(lineage.sources.contains_key(&SOURCE));

    lineage.observe(
        edges(),
        &full_members().iter().copied().collect(),
        &HashMap::new(),
        true,
    );
    assert!(lineage.sources.is_empty());

    lineage.observe(
        edges(),
        &full_members().iter().copied().collect(),
        &HashMap::new(),
        true,
    );
    assert!(lineage.sources.is_empty());
}

fn publish(lineage: &mut ExactContainmentLineage) {
    lineage.observe(&[], &HashSet::from([SOURCE]), &HashMap::new(), true);
}

fn observe(
    lineage: &mut ExactContainmentLineage,
    version: u64,
    active: &[u64],
    edges: &[LineageEdge],
    quiescent: bool,
) {
    lineage.observe(
        edges,
        &active.iter().copied().collect(),
        &HashMap::from([(SOURCE, version)]),
        quiescent,
    );
}

fn related(lineage: &ExactContainmentLineage, version: u64) -> HashMap<u64, u64> {
    lineage.related_sources(
        &HashMap::from([(SOURCE, version)]),
        &HashSet::from([TARGET]),
    )
}

fn full_members() -> &'static [u64] {
    &[SOURCE, INTERMEDIATE, TARGET]
}

fn edges() -> &'static [LineageEdge] {
    &[
        LineageEdge {
            child: INTERMEDIATE,
            parent: SOURCE,
        },
        LineageEdge {
            child: TARGET,
            parent: INTERMEDIATE,
        },
    ]
}
