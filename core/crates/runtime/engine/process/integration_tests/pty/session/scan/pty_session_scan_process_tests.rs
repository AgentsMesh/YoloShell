use super::process_test_support::ReadyProcess;
use super::*;
use crate::process::pty_session_proof::CapabilityScanStatus;
use portable_pty::{PtySize, native_pty_system};
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::process::Stdio;

const AVAILABLE_SCAN_CAPABILITY: &str = "available-live-scan";
const AVAILABLE_SCAN_SESSION_ID: u32 = 43_001;
const AVAILABLE_SCAN_ROWS: u16 = 24;
const AVAILABLE_SCAN_COLUMNS: u16 = 80;
const AVAILABLE_SCAN_PIXEL_SIZE: u16 = 0;
const EXITED_ROOT_LINEAGE: u64 = 43_002;
const ROOT_IS_LIVE: bool = false;
const ROOT_HAS_EXITED: bool = true;
const SCAN_ESCAPED_PROCESSES: bool = false;

#[test]
fn available_scan_matches_a_live_capability_and_exact_tty_holder() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: AVAILABLE_SCAN_ROWS,
            cols: AVAILABLE_SCAN_COLUMNS,
            pixel_width: AVAILABLE_SCAN_PIXEL_SIZE,
            pixel_height: AVAILABLE_SCAN_PIXEL_SIZE,
        })
        .unwrap();
    let path = pair.master.tty_name().unwrap();
    let proof =
        PtySessionProof::capture(path.clone(), AVAILABLE_SCAN_CAPABILITY.to_owned()).unwrap();
    let input = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(path)
        .unwrap();
    drop(pair);
    let child = ReadyProcess::spawn(AVAILABLE_SCAN_CAPABILITY, Stdio::from(input));
    let identity = crate::process::ProcessIdentity::capture(child.id()).unwrap();
    let entry = crate::process::inventory_test_support::entry(
        identity,
        u32::default(),
        AVAILABLE_SCAN_SESSION_ID,
    );
    let lineage = entry.lineage_id();

    let scan = inventory(entry)
        .macos_capability_candidates(
            &proof,
            &PtyCapabilityScanState::default(),
            &Default::default(),
            &Default::default(),
            identity.start_time,
            lineage,
            ROOT_IS_LIVE,
            SCAN_ESCAPED_PROCESSES,
        )
        .unwrap();

    assert_eq!(scan.candidates.observed, HashSet::from([lineage]));
    assert_eq!(scan.candidates.stable, HashSet::from([lineage]));
    assert_eq!(scan.candidates.status, CapabilityScanStatus::Complete);
}

#[test]
fn exited_root_scan_finds_an_escaped_capability_but_remains_pending() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: AVAILABLE_SCAN_ROWS,
            cols: AVAILABLE_SCAN_COLUMNS,
            pixel_width: AVAILABLE_SCAN_PIXEL_SIZE,
            pixel_height: AVAILABLE_SCAN_PIXEL_SIZE,
        })
        .unwrap();
    let proof = PtySessionProof::capture(
        pair.master.tty_name().unwrap(),
        AVAILABLE_SCAN_CAPABILITY.to_owned(),
    )
    .unwrap();
    let child = ReadyProcess::spawn(AVAILABLE_SCAN_CAPABILITY, Stdio::null());
    let identity = crate::process::ProcessIdentity::capture(child.id()).unwrap();
    let entry = crate::process::inventory_test_support::entry(
        identity,
        u32::default(),
        AVAILABLE_SCAN_SESSION_ID,
    );
    let lineage = entry.lineage_id();

    let scan = inventory(entry)
        .macos_capability_candidates(
            &proof,
            &PtyCapabilityScanState::default(),
            &Default::default(),
            &Default::default(),
            identity.start_time,
            EXITED_ROOT_LINEAGE,
            ROOT_HAS_EXITED,
            SCAN_ESCAPED_PROCESSES,
        )
        .unwrap();

    assert_eq!(scan.candidates.observed, HashSet::from([lineage]));
    assert_eq!(scan.candidates.stable, HashSet::from([lineage]));
    assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(scan.coverage_status, CapabilityScanStatus::Complete);
}

fn inventory(entry: InventoryEntry) -> StableInventory {
    let lineage = entry.lineage_id();
    StableInventory {
        entries: vec![entry],
        observed: vec![entry],
        by_pid: HashMap::from([(entry.pid, entry)]),
        by_live: HashMap::from([(entry.live_identity().unwrap(), entry)]),
        trusted_parents: HashMap::new(),
        possible_parents: HashMap::new(),
        observed_sessions: HashMap::new(),
        observed_lineages: HashSet::from([lineage]),
        stable_lineages: HashSet::from([lineage]),
        unstable_lineages: HashSet::new(),
    }
}
