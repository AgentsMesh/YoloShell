use super::super::super::macos_liveness::MacosGlobalScanMode;
use super::*;

const START_TIME_STEP: u64 = 1;
const PID_STEP: u32 = 1;
const EXPECTED_BOUNDARY_CANDIDATES: usize = 2;
const PRE_ROOT_PROOF_DETAIL: &str = "pre-root candidate reached capability proof";

#[test]
fn candidate_universe_orders_same_pid_by_generation() {
    let observed = vec![
        inventory_entry(FIRST_PID, SECOND_GENERATION),
        inventory_entry(FIRST_PID, FIRST_GENERATION),
    ];
    let stable = HashSet::from([FIRST_GENERATION]);

    let current =
        candidate_universe(&inventory(observed, stable), u64::default(), u64::default()).unwrap();
    let ordered = current
        .iter()
        .map(|candidate| (candidate.live.unique_id(), candidate.stable))
        .collect::<Vec<_>>();

    assert_eq!(
        ordered,
        [(FIRST_GENERATION, true), (SECOND_GENERATION, false)]
    );
}

#[test]
fn candidate_universe_excludes_pre_root_without_work() {
    let older = inventory_entry_at(
        FIRST_PID,
        FIRST_GENERATION,
        TEST_START_TIME - START_TIME_STEP,
    );
    let current = candidate_universe(
        &inventory(vec![older], HashSet::from([older.lineage_id()])),
        TEST_START_TIME,
        THIRD_GENERATION,
    )
    .unwrap();
    let scan = advance_scan(
        current,
        &ReleasedScanState::default(),
        ONE_CANDIDATE_PER_BATCH,
        |_candidate, _budget| panic!("{PRE_ROOT_PROOF_DETAIL}"),
    )
    .unwrap();

    assert_eq!(scan.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(scan.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
}

#[test]
fn candidate_universe_covers_boundary_and_post_root() {
    let boundary = inventory_entry_at(FIRST_PID, FIRST_GENERATION, TEST_START_TIME);
    let post_root = inventory_entry_at(
        SECOND_PID,
        SECOND_GENERATION,
        TEST_START_TIME + START_TIME_STEP,
    );
    let current = candidate_universe(
        &inventory(
            vec![boundary, post_root],
            HashSet::from([boundary.lineage_id(), post_root.lineage_id()]),
        ),
        TEST_START_TIME,
        boundary.lineage_id(),
    )
    .unwrap();
    let mut evaluated = Vec::new();
    let scan = advance_scan(
        current,
        &ReleasedScanState::default(),
        MAX_PTY_PROOF_CANDIDATES,
        |candidate, _budget| {
            evaluated.push(candidate.live);
            Ok(CandidateEvaluation::Complete(false))
        },
    )
    .unwrap();

    assert_eq!(scan.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(evaluated.len(), EXPECTED_BOUNDARY_CANDIDATES);
}

#[test]
fn candidate_universe_defers_post_root_generation_loss() {
    let post_root = inventory_entry_at(
        THIRD_PID,
        THIRD_GENERATION,
        TEST_START_TIME + START_TIME_STEP,
    );
    let current = candidate_universe(
        &inventory(vec![post_root], HashSet::from([post_root.lineage_id()])),
        TEST_START_TIME,
        FIRST_GENERATION,
    )
    .unwrap();
    let scan = advance_scan(
        current,
        &ReleasedScanState::default(),
        MAX_PTY_PROOF_CANDIDATES,
        |_candidate, _budget| Ok(CandidateEvaluation::Deferred),
    )
    .unwrap();

    assert_eq!(scan.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(scan.coverage_status, CapabilityScanStatus::Pending);
    assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
}

#[test]
fn candidate_universe_excludes_post_root_child_of_pre_root_parent() {
    let parent = inventory_entry_at(
        FIRST_PID,
        FIRST_GENERATION,
        TEST_START_TIME - START_TIME_STEP,
    );
    let mut child = inventory_entry_at(
        SECOND_PID,
        SECOND_GENERATION,
        TEST_START_TIME + START_TIME_STEP,
    );
    child.parent_unique_id = parent.lineage_id();
    let stable = HashSet::from([parent.lineage_id(), child.lineage_id()]);
    let current = candidate_universe(
        &inventory(vec![parent, child], stable),
        TEST_START_TIME,
        THIRD_GENERATION,
    )
    .unwrap();

    assert!(current.is_empty());
}

#[test]
fn parent_before_child_order_survives_reverse_pid_order() {
    let parent = inventory_entry(FIRST_PID, FIRST_GENERATION);
    let mut child = inventory_entry(FIRST_PID - PID_STEP, SECOND_GENERATION);
    child.parent_unique_id = parent.lineage_id();
    let current = candidate_universe(
        &inventory(vec![child, parent], HashSet::new()),
        u64::default(),
        THIRD_GENERATION,
    )
    .unwrap();
    let lineages = current
        .iter()
        .map(|candidate| candidate.live.lineage_id())
        .collect::<Vec<_>>();

    assert_eq!(lineages, [FIRST_GENERATION, SECOND_GENERATION]);
}

#[path = "../transferable/pty_session_transferable_universe_tests.rs"]
mod transferable_tests;
