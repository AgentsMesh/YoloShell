use super::*;
use crate::process::LiveProcessIdentity;
use std::cell::Cell;

const FIRST_PID: u32 = 42_001;
const SECOND_PID: u32 = 42_002;
const FIRST_GENERATION: u64 = 52_001;
const SECOND_GENERATION: u64 = 52_002;
const TEST_PROCESS_GROUP: u32 = 62_001;
const TEST_SESSION_ID: u32 = 62_002;
const TEST_START_TIME: u64 = 62_003;
const TEST_FILESYSTEM_DEVICE: u32 = 63_001;
const TEST_INODE: u64 = 63_002;
const REPLACEMENT_INODE: u64 = 63_003;
const TEST_TERMINAL_RDEV: u32 = 63_004;

#[test]
fn numeric_pid_reuse_changes_the_exact_holder_fence() {
    let before_pids = HashSet::from([FIRST_PID]);
    let after_pids = before_pids.clone();
    let before = HashSet::from([identity(FIRST_PID, FIRST_GENERATION)]);
    let after = HashSet::from([identity(FIRST_PID, SECOND_GENERATION)]);

    assert!(!available::available_fence_stable(
        path_identity(TEST_INODE),
        path_identity(TEST_INODE),
        &before_pids,
        &before,
        Some(path_identity(TEST_INODE)),
        Some(&after_pids),
        Some(&after),
    ));
}

#[test]
fn numeric_pid_match_does_not_cover_a_missing_exact_generation() {
    let observed = HashSet::from([identity(FIRST_PID, FIRST_GENERATION)]);
    let holder = HashSet::from([identity(FIRST_PID, SECOND_GENERATION)]);

    assert!(!available::inventory_covers_holders(&observed, &holder));
}

#[test]
fn stale_inventory_generation_never_reaches_the_proof_hook() {
    let observed = [inventory_entry(FIRST_PID, FIRST_GENERATION)];
    let stable = HashSet::from([FIRST_GENERATION]);
    let holders = HashSet::from([identity(FIRST_PID, SECOND_GENERATION)]);
    let queried = Cell::new(false);

    let candidates = evaluate_candidates(&observed, &stable, &holders, |_entry| {
        queried.set(true);
        Ok(false)
    })
    .unwrap();

    assert!(!queried.get());
    assert!(candidates.observed.is_empty());
    assert!(!available::inventory_covers_holders(
        &HashSet::from([identity(FIRST_PID, FIRST_GENERATION)]),
        &holders,
    ));
}

#[test]
fn holder_identity_capture_failure_is_not_a_partial_fence() {
    let pids = HashSet::from([FIRST_PID, SECOND_PID]);

    let captured = available::capture_holder_identities_with(&pids, |pid| {
        (pid == FIRST_PID).then(|| identity(FIRST_PID, FIRST_GENERATION))
    });

    assert!(captured.is_none());
}

#[test]
fn path_or_holder_set_change_invalidates_available_coverage() {
    let before_pids = HashSet::from([FIRST_PID]);
    let before = HashSet::from([identity(FIRST_PID, FIRST_GENERATION)]);
    let added_pids = HashSet::from([FIRST_PID, SECOND_PID]);
    let added = HashSet::from([
        identity(FIRST_PID, FIRST_GENERATION),
        identity(SECOND_PID, SECOND_GENERATION),
    ]);

    assert!(!available::available_fence_stable(
        path_identity(TEST_INODE),
        path_identity(TEST_INODE),
        &before_pids,
        &before,
        None,
        Some(&before_pids),
        Some(&before),
    ));
    assert!(!available::available_fence_stable(
        path_identity(TEST_INODE),
        path_identity(TEST_INODE),
        &before_pids,
        &before,
        Some(path_identity(TEST_INODE)),
        Some(&added_pids),
        Some(&added),
    ));
}

#[test]
fn unavailable_postscan_identity_capture_invalidates_available_coverage() {
    let pids = HashSet::from([FIRST_PID]);
    let holders = HashSet::from([identity(FIRST_PID, FIRST_GENERATION)]);

    assert!(!available::available_fence_stable(
        path_identity(TEST_INODE),
        path_identity(TEST_INODE),
        &pids,
        &holders,
        Some(path_identity(TEST_INODE)),
        Some(&pids),
        None,
    ));
}

#[test]
fn unchanged_path_and_exact_holders_complete_the_fence() {
    let pids = HashSet::from([FIRST_PID]);
    let holders = HashSet::from([identity(FIRST_PID, FIRST_GENERATION)]);

    assert!(available::available_fence_stable(
        path_identity(TEST_INODE),
        path_identity(TEST_INODE),
        &pids,
        &holders,
        Some(path_identity(TEST_INODE)),
        Some(&pids),
        Some(&holders),
    ));
}

#[test]
fn same_path_and_rdev_with_a_replaced_inode_invalidates_the_fence() {
    let pids = HashSet::from([FIRST_PID]);
    let holders = HashSet::from([identity(FIRST_PID, FIRST_GENERATION)]);

    assert!(!available::available_fence_stable(
        path_identity(TEST_INODE),
        path_identity(TEST_INODE),
        &pids,
        &holders,
        Some(path_identity(REPLACEMENT_INODE)),
        Some(&pids),
        Some(&holders),
    ));
}

#[test]
fn unstable_available_fence_preserves_positive_evidence_but_forces_pending() {
    let mut candidates = CapabilityCandidates::default();
    candidates.observed.insert(FIRST_GENERATION);
    candidates.stable.insert(FIRST_GENERATION);
    let mut result = available::finish_available_scan(candidates, false);

    available::require_stable_available_fence(&mut result, false);

    assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
    assert!(result.candidates.observed.contains(&FIRST_GENERATION));
    assert!(result.candidates.stable.contains(&FIRST_GENERATION));
}

fn identity(pid: u32, generation: u64) -> LiveProcessIdentity {
    LiveProcessIdentity::from_kernel(pid, generation).unwrap()
}

fn path_identity(inode: u64) -> super::super::macos_object_identity::MacosPtyObjectIdentity {
    super::super::macos_object_identity::MacosPtyObjectIdentity::from_kernel_fields(
        TEST_FILESYSTEM_DEVICE,
        inode,
        TEST_TERMINAL_RDEV,
    )
}

fn inventory_entry(pid: u32, generation: u64) -> InventoryEntry {
    InventoryEntry {
        pid,
        parent_pid: u32::default(),
        process_group: TEST_PROCESS_GROUP,
        session_id: TEST_SESSION_ID,
        start_time: TEST_START_TIME,
        unique_id: generation,
        parent_unique_id: u64::default(),
    }
}
