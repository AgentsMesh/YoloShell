use super::captured_lineage::{CapturedLineage, LineageEdge, LineageRetention};
use std::collections::{HashMap, HashSet};

const ROOT: u64 = 10;
const INTERMEDIATE: u64 = 20;
const LEAF: u64 = 30;
const ALTERNATE_PARENT: u64 = 40;
const REPLACEMENT: u64 = 50;
const NO_PARENT: u64 = 0;

#[test]
fn current_parent_reaches_an_exact_anchor() {
    let lineage = CapturedLineage::default();
    let current = HashMap::from([(LEAF, HashSet::from([ROOT]))]);

    assert!(lineage.reaches(LEAF, &HashSet::from([ROOT]), &current));
}

#[test]
fn historical_edge_bridges_a_missing_intermediate() {
    let mut lineage = CapturedLineage::default();
    lineage.extend([edge(INTERMEDIATE, ROOT)]);
    let current = HashMap::from([(LEAF, HashSet::from([INTERMEDIATE]))]);

    assert!(lineage.reaches(LEAF, &HashSet::from([ROOT]), &current));
}

#[test]
fn reparent_after_exec_appends_a_second_parent_without_conflict() {
    let mut lineage = CapturedLineage::default();
    lineage.extend([edge(INTERMEDIATE, ROOT)]);
    lineage.extend([edge(INTERMEDIATE, ALTERNATE_PARENT)]);

    assert!(lineage.reaches(INTERMEDIATE, &HashSet::from([ROOT]), &HashMap::new()));
    assert!(lineage.reaches(
        INTERMEDIATE,
        &HashSet::from([ALTERNATE_PARENT]),
        &HashMap::new()
    ));
}

#[test]
fn numeric_pid_replacement_does_not_inherit_old_unique_id_edges() {
    let mut lineage = CapturedLineage::default();
    lineage.extend([edge(INTERMEDIATE, ROOT)]);

    assert!(!lineage.reaches(REPLACEMENT, &HashSet::from([ROOT]), &HashMap::new()));
}

#[test]
fn cycles_and_zero_parent_terminate_without_authority() {
    let mut lineage = CapturedLineage::default();
    lineage.extend([
        edge(INTERMEDIATE, LEAF),
        edge(LEAF, INTERMEDIATE),
        edge(REPLACEMENT, NO_PARENT),
    ]);

    assert!(!lineage.reaches(LEAF, &HashSet::from([ROOT]), &HashMap::new()));
    assert!(!lineage.reaches(REPLACEMENT, &HashSet::from([ROOT]), &HashMap::new()));
}

#[test]
fn retain_paths_removes_history_above_an_exact_anchor() {
    let mut lineage = CapturedLineage::default();
    lineage.extend([edge(INTERMEDIATE, ROOT), edge(LEAF, INTERMEDIATE)]);

    let active = HashSet::from([LEAF]);
    let anchors = HashSet::from([INTERMEDIATE]);
    lineage.retain_paths(LineageRetention {
        active: &active,
        anchors: &anchors,
    });

    assert!(!lineage.reaches(INTERMEDIATE, &HashSet::from([ROOT]), &HashMap::new()));
    assert!(lineage.reaches(LEAF, &HashSet::from([INTERMEDIATE]), &HashMap::new()));
}

fn edge(child: u64, parent: u64) -> LineageEdge {
    LineageEdge { child, parent }
}
