use super::*;
use crate::process::pty_session_proof::CapabilityScanStatus;
use portable_pty::{PtySize, native_pty_system};
use std::cell::RefCell;

const HOLDER_PID: u32 = 41_001;
const UNRELATED_PID: u32 = 41_002;
const TEST_PROCESS_GROUP: u32 = 41_003;
const TEST_SESSION_ID: u32 = 41_004;
const TEST_START_TIME: u64 = 41_005;
const HOLDER_GENERATION: u64 = 41_006;
const UNRELATED_GENERATION: u64 = 41_007;
const HOLDER_ERROR_DETAIL: &str = "injected path-holder mapping failure";
const HOLDER_TEST_ROWS: u16 = 24;
const HOLDER_TEST_COLUMNS: u16 = 80;
const HOLDER_TEST_PIXEL_SIZE: u16 = 0;
const HOLDER_TEST_CAPABILITY: &str = "available-holder-scan";
const ROOT_EXITED: bool = false;
const SCAN_ESCAPED_PROCESSES: bool = false;

#[test]
fn path_holder_missing_from_inventory_keeps_available_scan_pending() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: HOLDER_TEST_ROWS,
            cols: HOLDER_TEST_COLUMNS,
            pixel_width: HOLDER_TEST_PIXEL_SIZE,
            pixel_height: HOLDER_TEST_PIXEL_SIZE,
        })
        .unwrap();
    let proof = PtySessionProof::capture(
        pair.master.tty_name().unwrap(),
        HOLDER_TEST_CAPABILITY.to_owned(),
    )
    .unwrap();

    let scan = empty_inventory()
        .macos_capability_candidates(
            &proof,
            &PtyCapabilityScanState::default(),
            &Default::default(),
            &Default::default(),
            u64::default(),
            u64::default(),
            ROOT_EXITED,
            SCAN_ESCAPED_PROCESSES,
        )
        .unwrap();

    assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(scan.coverage_status, CapabilityScanStatus::Pending);
    assert!(scan.candidates.observed.is_empty());
}

#[test]
fn deferred_available_candidate_marks_evidence_and_coverage_pending() {
    let mut candidates = CapabilityCandidates::default();
    candidates.observed.insert(HOLDER_GENERATION);
    candidates.stable.insert(HOLDER_GENERATION);
    let scan = super::available::finish_available_scan(candidates, true);

    assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(scan.coverage_status, CapabilityScanStatus::Pending);
    assert!(scan.candidates.observed.contains(&HOLDER_GENERATION));
    assert!(scan.candidates.stable.contains(&HOLDER_GENERATION));
}

#[test]
fn unrelated_inventory_process_never_reaches_the_proof_hook() {
    let observed = [
        inventory_entry(HOLDER_PID, HOLDER_GENERATION),
        inventory_entry(UNRELATED_PID, UNRELATED_GENERATION),
    ];
    let stable = observed.iter().map(|entry| entry.lineage_id()).collect();
    let holders = HashSet::from([observed[0].live_identity().unwrap()]);
    let queried = RefCell::new(Vec::new());

    let candidates = evaluate_candidates(&observed, &stable, &holders, |entry| {
        queried.borrow_mut().push(entry.pid);
        Ok(false)
    })
    .unwrap();

    assert!(candidates.observed.is_empty());
    assert!(candidates.stable.is_empty());
    assert_eq!(queried.into_inner(), [HOLDER_PID]);
}

#[test]
fn released_path_candidate_set_checks_every_observed_process() {
    let observed = [
        inventory_entry(HOLDER_PID, HOLDER_GENERATION),
        inventory_entry(UNRELATED_PID, UNRELATED_GENERATION),
    ];
    let stable = observed.iter().map(|entry| entry.lineage_id()).collect();
    let candidates = observed
        .iter()
        .map(|entry| entry.live_identity().unwrap())
        .collect();
    let queried = RefCell::new(Vec::new());

    evaluate_candidates(&observed, &stable, &candidates, |entry| {
        queried.borrow_mut().push(entry.pid);
        Ok(false)
    })
    .unwrap();

    let mut queried = queried.into_inner();
    queried.sort_unstable();
    assert_eq!(queried, [HOLDER_PID, UNRELATED_PID]);
}

#[test]
fn every_path_holder_failure_maps_to_its_capability_error_class() {
    let query = map_path_holder_error(PathHolderError::Query(std::io::Error::from_raw_os_error(
        libc::EIO,
    )));
    assert!(matches!(
        query,
        CapabilityProofError::HolderInventoryQuery(_)
    ));

    let budget = map_path_holder_error(PathHolderError::BudgetExceeded(HOLDER_ERROR_DETAIL));
    assert!(matches!(
        budget,
        CapabilityProofError::ScanBudgetExceeded(_)
    ));

    let invariant = map_path_holder_error(PathHolderError::Invariant(HOLDER_ERROR_DETAIL));
    assert!(matches!(invariant, CapabilityProofError::Invariant(_)));
}

fn inventory_entry(pid: u32, unique_id: u64) -> InventoryEntry {
    InventoryEntry {
        pid,
        parent_pid: u32::default(),
        process_group: TEST_PROCESS_GROUP,
        session_id: TEST_SESSION_ID,
        start_time: TEST_START_TIME,
        unique_id,
        parent_unique_id: u64::default(),
    }
}

fn empty_inventory() -> StableInventory {
    StableInventory {
        entries: Vec::new(),
        observed: Vec::new(),
        by_pid: HashMap::new(),
        by_live: HashMap::new(),
        trusted_parents: HashMap::new(),
        possible_parents: HashMap::new(),
        observed_sessions: HashMap::new(),
        observed_lineages: HashSet::new(),
        stable_lineages: HashSet::new(),
        unstable_lineages: HashSet::new(),
    }
}
