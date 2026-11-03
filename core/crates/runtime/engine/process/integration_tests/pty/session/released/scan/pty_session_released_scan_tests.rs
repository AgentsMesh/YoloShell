use super::super::macos_scan::process_test_support::ReadyProcess;
use super::universe::{candidate_universe, transferable_candidate_universe};
use super::*;
use crate::process::inventory_snapshot::InventoryEntry;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;

const FIRST_PID: u32 = 41_101;
const SECOND_PID: u32 = 41_102;
const THIRD_PID: u32 = 41_103;
const TEST_PROCESS_GROUP: u32 = 41_104;
const TEST_SESSION_ID: u32 = 41_105;
const TEST_START_TIME: u64 = 41_106;
const FIRST_GENERATION: u64 = 51_101;
const SECOND_GENERATION: u64 = 51_102;
const THIRD_GENERATION: u64 = 51_103;
const FOURTH_GENERATION: u64 = 51_104;
const ONE_CANDIDATE_PER_BATCH: usize = 1;
const EMPTY_SCAN_TTY_PATH: &str = "/dev/null";
const EMPTY_SCAN_CAPABILITY: &str = "released-empty-scan";
const LIVE_SCAN_CAPABILITY: &str = "released-live-scan";
const LIVE_SCAN_SESSION_ID: u32 = 41_107;

#[test]
fn released_scan_keeps_an_empty_inventory_pending_without_global_exclusion() {
    let proof = PtySessionProof::capture(
        PathBuf::from(EMPTY_SCAN_TTY_PATH),
        EMPTY_SCAN_CAPABILITY.to_owned(),
    )
    .unwrap();
    let state = ReleasedScanState::default();

    let scan = scan_released(
        &inventory(Vec::new(), HashSet::new()),
        &proof,
        &state,
        u64::default(),
        u64::default(),
        MacosGlobalScanMode::LineageEnvironment,
        &HashSet::new(),
    )
    .unwrap();

    assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(scan.coverage_status, CapabilityScanStatus::Complete);
    assert!(scan.candidates.observed.is_empty());
    assert!(scan.candidates.stable.is_empty());
}

#[test]
fn released_scan_matches_a_live_capability_and_tty_descriptor() {
    let child = ReadyProcess::spawn(LIVE_SCAN_CAPABILITY, Stdio::null());
    let identity = crate::process::ProcessIdentity::capture(child.id()).unwrap();
    let entry = crate::process::inventory_test_support::entry(
        identity,
        u32::default(),
        LIVE_SCAN_SESSION_ID,
    );
    let lineage = entry.lineage_id();
    let proof = PtySessionProof::capture(
        PathBuf::from(EMPTY_SCAN_TTY_PATH),
        LIVE_SCAN_CAPABILITY.to_owned(),
    )
    .unwrap();

    let scan = scan_released(
        &inventory(vec![entry], HashSet::from([lineage])),
        &proof,
        &ReleasedScanState::default(),
        u64::default(),
        u64::default(),
        MacosGlobalScanMode::LineageEnvironment,
        &HashSet::new(),
    )
    .unwrap();

    assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(scan.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(scan.candidates.observed, HashSet::from([lineage]));
    assert_eq!(scan.candidates.stable, HashSet::from([lineage]));
}

fn candidate(pid: u32, generation: u64, stable: bool) -> ReleasedCandidate {
    ReleasedCandidate {
        live: LiveProcessIdentity::from_kernel(pid, generation).unwrap(),
        durable: crate::process::ProcessIdentity {
            pid,
            start_time: TEST_START_TIME,
        },
        parent_lineage: u64::default(),
        stable,
        capability_root: false,
        environment_eligible: true,
    }
}

fn child_candidate(
    pid: u32,
    generation: u64,
    parent_lineage: u64,
    capability_root: bool,
) -> ReleasedCandidate {
    ReleasedCandidate {
        live: LiveProcessIdentity::from_kernel(pid, generation).unwrap(),
        durable: crate::process::ProcessIdentity {
            pid,
            start_time: TEST_START_TIME,
        },
        parent_lineage,
        stable: false,
        capability_root,
        environment_eligible: true,
    }
}

fn candidates<const N: usize>(items: [ReleasedCandidate; N]) -> Arc<[ReleasedCandidate]> {
    Arc::from(items)
}

fn commit(state: &mut ReleasedScanState, result: CapabilityScanResult) -> CapabilityCandidates {
    let CapabilityScanResult {
        candidates,
        advance,
        ..
    } = result;
    *state = advance.next.released;
    candidates
}

fn inventory_entry(pid: u32, generation: u64) -> InventoryEntry {
    inventory_entry_at(pid, generation, TEST_START_TIME)
}

fn inventory_entry_at(pid: u32, generation: u64, start_time: u64) -> InventoryEntry {
    InventoryEntry {
        pid,
        parent_pid: u32::default(),
        process_group: TEST_PROCESS_GROUP,
        session_id: TEST_SESSION_ID,
        start_time,
        unique_id: generation,
        parent_unique_id: u64::default(),
    }
}

fn inventory(observed: Vec<InventoryEntry>, stable_lineages: HashSet<u64>) -> StableInventory {
    let entries = observed
        .iter()
        .copied()
        .filter(|entry| stable_lineages.contains(&entry.lineage_id()))
        .collect();
    let observed_lineages: HashSet<_> = observed.iter().map(|entry| entry.lineage_id()).collect();
    let unstable_lineages = observed_lineages
        .difference(&stable_lineages)
        .copied()
        .collect();
    StableInventory {
        entries,
        observed,
        by_pid: HashMap::new(),
        by_live: HashMap::new(),
        trusted_parents: HashMap::new(),
        possible_parents: HashMap::new(),
        observed_sessions: HashMap::new(),
        observed_lineages,
        stable_lineages,
        unstable_lineages,
    }
}

include!("../holder/pty_session_released_holder_test_support.rs");
include!("pty_session_released_scan_test_wiring.rs");
