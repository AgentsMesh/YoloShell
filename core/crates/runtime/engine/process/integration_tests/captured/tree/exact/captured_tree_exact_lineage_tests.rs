use super::ExactContainmentLineage;
use crate::process::captured_lineage::LineageEdge;
use std::collections::{HashMap, HashSet};

const SOURCE: u64 = 11;
const INTERMEDIATE: u64 = 12;
const TARGET: u64 = 13;
const UNRELATED: u64 = 14;
const FIRST_VERSION: u64 = 1;
const SECOND_VERSION: u64 = 2;

#[test]
fn ancestry_without_a_prior_publication_never_mints_a_source() {
    let mut lineage = ExactContainmentLineage::default();
    observe(&mut lineage, FIRST_VERSION, &[], edges(), true);

    assert!(related(&lineage, FIRST_VERSION).is_empty());
}

#[test]
fn a_preexisting_direct_child_cannot_satisfy_later_fork_debt() {
    let mut lineage = ExactContainmentLineage::default();
    publish(&mut lineage, &[SOURCE, INTERMEDIATE, TARGET], edges());
    observe(&mut lineage, FIRST_VERSION, &[], edges(), true);

    assert!(related(&lineage, FIRST_VERSION).is_empty());
}

#[test]
fn source_target_cannot_bypass_a_fresh_child_proof() {
    let mut lineage = ExactContainmentLineage::default();
    publish(&mut lineage, &[SOURCE], &[]);
    observe(&mut lineage, FIRST_VERSION, &[], &[], true);

    assert!(
        lineage
            .related_sources(
                &HashMap::from([(SOURCE, FIRST_VERSION)]),
                &HashSet::from([SOURCE]),
            )
            .is_empty()
    );
}

#[test]
fn a_fresh_multihop_child_binds_only_its_source() {
    let mut lineage = ExactContainmentLineage::default();
    publish(&mut lineage, &[SOURCE, UNRELATED], &[]);
    lineage.observe(
        edges(),
        &HashSet::from([SOURCE, INTERMEDIATE, TARGET, UNRELATED]),
        &HashMap::from([(SOURCE, FIRST_VERSION), (UNRELATED, FIRST_VERSION)]),
        true,
    );

    assert_eq!(
        related(&lineage, FIRST_VERSION),
        HashMap::from([(SOURCE, FIRST_VERSION)])
    );
}

#[test]
fn a_later_fork_version_cannot_reuse_an_earlier_child() {
    let mut lineage = ExactContainmentLineage::default();
    publish(&mut lineage, &[SOURCE], &[]);
    observe(
        &mut lineage,
        FIRST_VERSION,
        &[SOURCE, INTERMEDIATE, TARGET],
        edges(),
        true,
    );
    observe(
        &mut lineage,
        SECOND_VERSION,
        &[SOURCE, INTERMEDIATE, TARGET],
        edges(),
        true,
    );

    assert!(related(&lineage, SECOND_VERSION).is_empty());
}

#[test]
fn an_unquiescent_inventory_cannot_publish_ancestry() {
    let mut lineage = ExactContainmentLineage::default();
    publish(&mut lineage, &[SOURCE], &[]);
    observe(
        &mut lineage,
        FIRST_VERSION,
        &[SOURCE, INTERMEDIATE, TARGET],
        edges(),
        false,
    );

    assert!(related(&lineage, FIRST_VERSION).is_empty());
}

fn publish(lineage: &mut ExactContainmentLineage, active: &[u64], edges: &[LineageEdge]) {
    lineage.observe(
        edges,
        &active.iter().copied().collect(),
        &HashMap::new(),
        true,
    );
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
        &HashMap::from([(SOURCE, version), (UNRELATED, version)]),
        &HashSet::from([TARGET]),
    )
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
