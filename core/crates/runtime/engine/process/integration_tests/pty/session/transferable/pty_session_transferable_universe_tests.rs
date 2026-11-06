use super::*;

const EXPECTED_EFFECTIVE_UID_CANDIDATES: usize = 3;
const EXPECTED_PARENT_AND_CHILD_CANDIDATES: usize = 2;

#[test]
fn transferable_modes_route_a_foreign_pre_root_holder() {
    let older = inventory_entry_at(
        FIRST_PID,
        FIRST_GENERATION,
        TEST_START_TIME - START_TIME_STEP,
    );
    let snapshot = inventory(vec![older], HashSet::from([older.lineage_id()]));

    let present = scan_candidate_universe(
        &snapshot,
        MacosGlobalScanMode::TransferableWriterPresent,
        TEST_START_TIME,
        THIRD_GENERATION,
    )
    .unwrap();
    assert_eq!(present.len(), ONE_CANDIDATE_PER_BATCH);
    assert!(!present.first().unwrap().environment_eligible);
    assert_eq!(
        present.first().unwrap().live.lineage_id(),
        older.lineage_id()
    );
    let released = scan_candidate_universe(
        &snapshot,
        MacosGlobalScanMode::TransferableWriterReleased,
        TEST_START_TIME,
        THIRD_GENERATION,
    )
    .unwrap();
    assert_eq!(released.len(), ONE_CANDIDATE_PER_BATCH);
    assert!(!released.first().unwrap().environment_eligible);
}

#[test]
fn writer_release_retains_the_full_transferable_universe() {
    let parent = inventory_entry_at(
        SECOND_PID,
        FIRST_GENERATION,
        TEST_START_TIME - START_TIME_STEP,
    );
    let mut child = inventory_entry_at(
        FIRST_PID,
        SECOND_GENERATION,
        TEST_START_TIME + START_TIME_STEP,
    );
    child.parent_unique_id = parent.lineage_id();
    let snapshot = inventory(
        vec![parent, child],
        HashSet::from([parent.lineage_id(), child.lineage_id()]),
    );

    let lineage = scan_candidate_universe(
        &snapshot,
        MacosGlobalScanMode::LineageEnvironment,
        TEST_START_TIME,
        THIRD_GENERATION,
    )
    .unwrap();
    assert!(lineage.is_empty());
    let present = scan_candidate_universe(
        &snapshot,
        MacosGlobalScanMode::TransferableWriterPresent,
        TEST_START_TIME,
        THIRD_GENERATION,
    )
    .unwrap();
    assert_eq!(present.len(), EXPECTED_PARENT_AND_CHILD_CANDIDATES);
    let released = scan_candidate_universe(
        &snapshot,
        MacosGlobalScanMode::TransferableWriterReleased,
        TEST_START_TIME,
        THIRD_GENERATION,
    )
    .unwrap();
    assert_eq!(released.len(), EXPECTED_PARENT_AND_CHILD_CANDIDATES);
}

#[test]
fn transferable_universe_retains_the_full_effective_uid_inventory() {
    let parent = inventory_entry_at(
        THIRD_PID,
        FIRST_GENERATION,
        TEST_START_TIME - START_TIME_STEP,
    );
    let mut reparented = inventory_entry_at(
        FIRST_PID,
        SECOND_GENERATION,
        TEST_START_TIME + START_TIME_STEP,
    );
    reparented.parent_unique_id = parent.lineage_id();
    let unrelated = inventory_entry_at(SECOND_PID, THIRD_GENERATION, TEST_START_TIME);
    let stable = HashSet::from([
        parent.lineage_id(),
        reparented.lineage_id(),
        unrelated.lineage_id(),
    ]);

    let current = transferable_candidate_universe(
        &inventory(vec![parent, unrelated, reparented], stable),
        TEST_START_TIME,
        u64::default(),
    )
    .unwrap();
    let lineages = current
        .iter()
        .map(|candidate| candidate.live.lineage_id())
        .collect::<Vec<_>>();

    assert_eq!(current.len(), EXPECTED_EFFECTIVE_UID_CANDIDATES);
    assert_eq!(
        lineages,
        [
            unrelated.lineage_id(),
            parent.lineage_id(),
            reparented.lineage_id()
        ]
    );
}

#[test]
fn transferable_root_is_explicit_in_the_full_candidate_universe() {
    let root = inventory_entry_at(
        THIRD_PID,
        FIRST_GENERATION,
        TEST_START_TIME - START_TIME_STEP,
    );
    let mut child = inventory_entry_at(
        FIRST_PID,
        SECOND_GENERATION,
        TEST_START_TIME + START_TIME_STEP,
    );
    child.parent_unique_id = root.lineage_id();
    let unrelated = inventory_entry_at(SECOND_PID, THIRD_GENERATION, TEST_START_TIME);
    let stable = HashSet::from([
        root.lineage_id(),
        child.lineage_id(),
        unrelated.lineage_id(),
    ]);
    let current = transferable_candidate_universe(
        &inventory(vec![root, child, unrelated], stable),
        root.start_time,
        root.lineage_id(),
    )
    .unwrap();

    let root_candidate = current
        .iter()
        .find(|candidate| candidate.live.lineage_id() == root.lineage_id())
        .unwrap();
    assert!(root_candidate.capability_root);
    assert!(root_candidate.environment_eligible);
    let lineages = current
        .iter()
        .map(|candidate| candidate.live.lineage_id())
        .collect::<HashSet<_>>();
    assert_eq!(
        lineages,
        HashSet::from([
            root.lineage_id(),
            child.lineage_id(),
            unrelated.lineage_id()
        ])
    );
    assert!(
        current
            .iter()
            .find(|candidate| candidate.live.lineage_id() == child.lineage_id())
            .unwrap()
            .environment_eligible
    );
    assert!(
        !current
            .iter()
            .find(|candidate| candidate.live.lineage_id() == unrelated.lineage_id())
            .unwrap()
            .environment_eligible
    );
}
