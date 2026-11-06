use super::*;
use crate::process::constants::PTY_SESSION_CAPABILITY_ENV;
use crate::process::inventory_test_support::entry;
use crate::process::{
    MacosProcessCapabilityIssuer, MacosProcessCapabilityRelease, ProcessIdentity,
};
use portable_pty::{PtySize, native_pty_system};
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};

const TEST_CAPABILITY: &str = "released-real-pty-holder";
const HOLDER_EXECUTABLE: &str = "/bin/sleep";
const HOLDER_DURATION: &str = "30";
const TEST_SESSION: u32 = 84_001;
const ABSENT_ROOT_LINEAGE: u64 = u64::MAX;
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const QUEUED_RIGHT_MARKER: u8 = b'P';
const REPEATED_QUEUED_SWEEPS: usize = 3;
const SCAN_ATTEMPT_LIMIT: usize = 64;
const MATERIALIZED_RIGHT_DETAIL: &str = "materialized PTY right was not discovered";

#[test]
fn bound_released_scan_finds_environment_scrubbed_exact_holder() {
    let pair = open_pty();
    let path = pair.master.tty_name().unwrap();
    let input = open_slave(&path);
    drop(pair.slave);
    let mut command = Command::new(HOLDER_EXECUTABLE);
    command
        .arg(HOLDER_DURATION)
        .env_remove(PTY_SESSION_CAPABILITY_ENV)
        .stdin(Stdio::from(input))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = crate::process::spawn_waitable_test_child(command);
    let child = ChildGuard(child);
    let identity = ProcessIdentity::capture(child.0.id()).unwrap();
    let process = entry(identity, u32::default(), TEST_SESSION);
    let proof = released_bound_proof(pair.master.as_ref());

    let scan = scan_released(
        &inventory(vec![process], HashSet::from([process.lineage_id()])),
        &proof,
        &ReleasedScanState::default(),
        identity.start_time,
        ABSENT_ROOT_LINEAGE,
        MacosGlobalScanMode::TransferableWriterReleased,
        &HashSet::new(),
    )
    .unwrap();

    assert_eq!(scan.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(
        scan.candidates.observed,
        HashSet::from([process.lineage_id()])
    );
}

#[test]
fn queued_pty_right_defeats_every_sequential_negative_sweep() {
    let pair = open_pty();
    let path = pair.master.tty_name().unwrap();
    let queued = open_slave(&path);
    drop(pair.slave);
    let proof = released_bound_proof(pair.master.as_ref());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let process = entry(identity, u32::default(), TEST_SESSION);
    let snapshot = inventory(vec![process], HashSet::from([process.lineage_id()]));
    let (sender, receiver) = UnixStream::pair().unwrap();
    crate::local_unix::test_support::send_right(
        sender.as_raw_fd(),
        queued.as_raw_fd(),
        QUEUED_RIGHT_MARKER,
    );
    drop(queued);
    let mut state = ReleasedScanState::default();
    let mut completed_sweeps = usize::default();

    for _attempt in 0..SCAN_ATTEMPT_LIMIT {
        let scan = released_scan(&snapshot, &proof, &state, identity);
        assert!(scan.candidates.observed.is_empty());
        assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
        completed_sweeps += usize::from(scan.coverage_status == CapabilityScanStatus::Complete);
        commit(&mut state, scan);
        if completed_sweeps == REPEATED_QUEUED_SWEEPS {
            break;
        }
    }
    assert_eq!(completed_sweeps, REPEATED_QUEUED_SWEEPS);

    let _received = crate::local_unix::macos::receive_exactly_one_right(
        receiver.as_raw_fd(),
        QUEUED_RIGHT_MARKER,
    )
    .unwrap();
    let mut materialized = None;
    for _attempt in 0..SCAN_ATTEMPT_LIMIT {
        let scan = released_scan(&snapshot, &proof, &state, identity);
        assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
        if scan.candidates.observed.contains(&process.lineage_id()) {
            materialized = Some(process.lineage_id());
        }
        commit(&mut state, scan);
        if materialized.is_some() {
            break;
        }
    }
    assert_eq!(
        materialized.expect(MATERIALIZED_RIGHT_DETAIL),
        process.lineage_id()
    );
}

fn released_scan(
    inventory: &StableInventory,
    proof: &PtySessionProof,
    state: &ReleasedScanState,
    root: ProcessIdentity,
) -> CapabilityScanResult {
    scan_released(
        inventory,
        proof,
        state,
        root.start_time,
        ABSENT_ROOT_LINEAGE,
        MacosGlobalScanMode::TransferableWriterReleased,
        &HashSet::new(),
    )
    .unwrap()
}

fn released_bound_proof(master: &dyn portable_pty::MasterPty) -> PtySessionProof {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    let mut proof = PtySessionProof::capture_bound(master, TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader.clone()).unwrap();
    drop(writer);
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
    drop(spawn_guard);
    proof
}

fn open_pty() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TERMINAL_ROWS,
            cols: TERMINAL_COLUMNS,
            pixel_width: TERMINAL_PIXEL_SIZE,
            pixel_height: TERMINAL_PIXEL_SIZE,
        })
        .unwrap()
}

fn open_slave(path: &std::path::Path) -> std::fs::File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(path)
        .unwrap()
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
