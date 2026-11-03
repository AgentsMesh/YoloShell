use super::*;

const GUARDIAN: u64 = 30;
const PARENT: u64 = 20;
const ROOT: u64 = 10;
const UNKNOWN: u64 = 40;
const NO_PARENT: u64 = 0;
const CANDIDATE_PID: u32 = 71_001;
const CANDIDATE_PROCESS_GROUP: u32 = 71_002;
const CANDIDATE_SESSION: u32 = 71_003;
const START_TIME_STEP: u64 = 1;

#[test]
fn proven_guardian_lineage_excludes_guardian_and_every_ancestor() {
    let observed = parents([(GUARDIAN, PARENT), (PARENT, ROOT), (ROOT, NO_PARENT)]);
    let possible = parents([(GUARDIAN, PARENT), (PARENT, ROOT)]);
    let trusted = possible.clone();

    assert_eq!(
        proven_ancestor_prefix(GUARDIAN, &observed, &possible, &trusted),
        (HashSet::from([GUARDIAN, PARENT, ROOT]), true)
    );
}

#[test]
fn missing_ancestor_evidence_retains_the_proven_prefix() {
    let observed = parents([(GUARDIAN, PARENT)]);
    let possible = observed.clone();
    let trusted = observed.clone();

    assert_eq!(
        proven_ancestor_prefix(GUARDIAN, &observed, &possible, &trusted),
        (HashSet::from([GUARDIAN, PARENT]), false)
    );
}

#[test]
fn ambiguous_ancestor_evidence_retains_only_the_proven_prefix() {
    let mut observed = parents([(GUARDIAN, PARENT), (PARENT, ROOT), (ROOT, NO_PARENT)]);
    observed.get_mut(&PARENT).unwrap().insert(UNKNOWN);
    let possible = observed.clone();
    let trusted = parents([(GUARDIAN, PARENT)]);

    assert_eq!(
        proven_ancestor_prefix(GUARDIAN, &observed, &possible, &trusted),
        (HashSet::from([GUARDIAN, PARENT]), false)
    );
}

#[test]
fn ancestor_cycle_fails_closed() {
    let observed = parents([(GUARDIAN, PARENT), (PARENT, GUARDIAN)]);
    let possible = observed.clone();
    let trusted = observed.clone();

    assert_eq!(
        proven_ancestor_prefix(GUARDIAN, &observed, &possible, &trusted),
        (HashSet::from([GUARDIAN, PARENT]), false)
    );
}

#[test]
fn newer_holder_is_containable_when_guardian_ancestry_is_incomplete() {
    let ancestry = incomplete_ancestry();
    let holder = candidate(
        UNKNOWN,
        ancestry.guardian.start_time.saturating_add(START_TIME_STEP),
    );

    assert_eq!(
        ancestry.classify(holder).unwrap(),
        HolderDisposition::Contain
    );
}

#[test]
fn unproven_older_holder_is_rejected_when_guardian_ancestry_is_incomplete() {
    let ancestry = incomplete_ancestry();
    let holder = candidate(
        UNKNOWN,
        ancestry.guardian.start_time.saturating_sub(START_TIME_STEP),
    );

    assert!(ancestry.classify(holder).is_err());
}

#[test]
fn proven_guardian_ancestor_is_never_a_containment_target() {
    let ancestry = incomplete_ancestry();
    let holder = candidate(
        PARENT,
        ancestry.guardian.start_time.saturating_sub(START_TIME_STEP),
    );

    assert_eq!(
        ancestry.classify(holder).unwrap(),
        HolderDisposition::Exclude
    );
}

fn incomplete_ancestry() -> GuardianAncestry {
    GuardianAncestry {
        guardian: ProcessIdentity::capture(std::process::id()).unwrap(),
        guardian_lineage: GUARDIAN,
        protected: HashSet::from([GUARDIAN, PARENT]),
        complete: false,
    }
}

fn candidate(lineage: u64, start_time: u64) -> InventoryEntry {
    InventoryEntry {
        pid: CANDIDATE_PID,
        parent_pid: u32::try_from(NO_PARENT).unwrap(),
        process_group: CANDIDATE_PROCESS_GROUP,
        session_id: CANDIDATE_SESSION,
        start_time,
        unique_id: lineage,
        parent_unique_id: NO_PARENT,
    }
}

fn parents<const N: usize>(edges: [(u64, u64); N]) -> ParentCandidates {
    let mut parents = ParentCandidates::new();
    for (child, parent) in edges {
        parents.entry(child).or_default().insert(parent);
    }
    parents
}
