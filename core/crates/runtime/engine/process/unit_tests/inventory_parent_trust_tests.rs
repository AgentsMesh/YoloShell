use super::*;

const ROOT: u64 = 100;
const INTERMEDIATE: u64 = 101;
const CHILD: u64 = 102;

#[test]
fn stable_unambiguous_edges_are_trusted() {
    let possible = ParentCandidates::from([
        (INTERMEDIATE, HashSet::from([ROOT])),
        (CHILD, HashSet::from([INTERMEDIATE])),
    ]);
    let stable = HashSet::from([ROOT, INTERMEDIATE, CHILD]);
    assert_eq!(trusted_parents(&possible, &stable), possible);
}

#[test]
fn competing_parent_evidence_is_not_trusted() {
    let possible = ParentCandidates::from([(CHILD, HashSet::from([ROOT, INTERMEDIATE]))]);
    assert!(trusted_parents(&possible, &HashSet::from([ROOT, INTERMEDIATE, CHILD])).is_empty());
}

#[test]
#[cfg(target_os = "linux")]
fn numeric_ancestry_requires_both_process_generations_to_be_stable() {
    let possible = ParentCandidates::from([
        (INTERMEDIATE, HashSet::from([ROOT])),
        (CHILD, HashSet::from([INTERMEDIATE])),
    ]);
    assert!(trusted_parents(&possible, &HashSet::from([ROOT, CHILD])).is_empty());
    assert!(trusted_parents(&possible, &HashSet::new()).is_empty());
}
