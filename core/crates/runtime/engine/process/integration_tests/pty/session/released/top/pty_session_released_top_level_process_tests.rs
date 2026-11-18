use super::*;
use crate::process::inventory_test_support::{TestProcess, entry};
use crate::process::{
    MacosProcessCapabilityIssuer, MacosProcessCapabilityObservation, PTY_SESSION_CAPABILITY_ENV,
    ProcessIdentity,
};
use portable_pty::{PtySize, native_pty_system};
use std::fs::OpenOptions;
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};
#[path = "pty_session_released_top_level_inventory_tests.rs"]
mod inventory_support;
use inventory_support::{empty_inventory, stable_inventory};

const TEST_CAPABILITY: &str = "released-top-level-queued-pty";
const TEST_SESSION: u32 = 85_001;
const ABSENT_ROOT_LINEAGE: u64 = u64::MAX;
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const QUEUED_RIGHT_MARKER: u8 = b'Q';
const RELEASE_TIMEOUT: Duration = Duration::from_secs(5);
const RELEASE_POLL_INTERVAL: Duration = Duration::from_millis(1);
const MATERIALIZED_RIGHT_DETAIL: &str = "top-level scan missed a materialized PTY right";
const RELEASE_TIMEOUT_DETAIL: &str = "process capability did not release after exact root exit";
const SCAN_TIMEOUT_DETAIL: &str = "released descriptor scan did not complete";

#[test]
fn available_path_cannot_hide_a_queued_pty_right_after_writer_release() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let path = pair.master.tty_name().unwrap();
    let queued = open_slave(&path);
    drop(pair.slave);
    let (proof, root) = released_bound_proof(pair.master.as_ref());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let process = entry(identity, u32::default(), TEST_SESSION);
    let inventory = stable_inventory(process);
    let (sender, receiver) = UnixStream::pair().unwrap();
    crate::local_unix::test_support::send_right(
        sender.as_raw_fd(),
        queued.as_raw_fd(),
        QUEUED_RIGHT_MARKER,
    );
    drop(queued);
    let mut state = PtyCapabilityScanState::default();

    let initial = scan(&inventory, &proof, &mut state, root);
    assert_eq!(initial.candidates.status, CapabilityScanStatus::Pending);
    let deadline = Instant::now() + RELEASE_TIMEOUT;
    let negative = loop {
        let result = scan(&inventory, &proof, &mut state, root);
        assert!(result.candidates.observed.is_empty());
        assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
        if result.coverage == CapabilityScanStatus::Complete {
            break result;
        }
        assert!(Instant::now() < deadline, "{SCAN_TIMEOUT_DETAIL}");
        std::thread::sleep(RELEASE_POLL_INTERVAL);
    };
    assert_eq!(negative.coverage, CapabilityScanStatus::Complete);

    let received = receive_right(&receiver);
    let mut materialized = false;
    let deadline = Instant::now() + RELEASE_TIMEOUT;
    loop {
        let result = scan(&inventory, &proof, &mut state, root);
        materialized |= result.candidates.observed.contains(&process.lineage_id());
        assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
        if materialized {
            break;
        }
        assert!(Instant::now() < deadline, "{SCAN_TIMEOUT_DETAIL}");
        std::thread::sleep(RELEASE_POLL_INTERVAL);
    }
    assert!(materialized, "{MATERIALIZED_RIGHT_DETAIL}");
    drop(received);

    let retained = scan(&empty_inventory(), &proof, &mut state, root);
    assert!(retained.candidates.observed.contains(&process.lineage_id()));
    assert_eq!(retained.candidates.status, CapabilityScanStatus::Pending);
}

struct ScanEvidence {
    candidates: CapabilityCandidates,
    coverage: CapabilityScanStatus,
}

fn scan(
    inventory: &StableInventory,
    proof: &PtySessionProof,
    state: &mut PtyCapabilityScanState,
    root: ProcessIdentity,
) -> ScanEvidence {
    let result = inventory
        .macos_capability_candidates(
            proof,
            state,
            &Default::default(),
            &Default::default(),
            root.start_time,
            ABSENT_ROOT_LINEAGE,
            false,
            false,
        )
        .unwrap();
    let CapabilityScanResult {
        candidates,
        advance,
        scheduling_status: _,
        coverage_status,
    } = result;
    state.commit(advance);
    ScanEvidence {
        candidates,
        coverage: coverage_status,
    }
}

fn released_bound_proof(
    master: &dyn portable_pty::MasterPty,
) -> (PtySessionProof, ProcessIdentity) {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    let mut root = TestProcess::spawn_with_environment_and_process_capability(
        PTY_SESSION_CAPABILITY_ENV,
        TEST_CAPABILITY,
        &writer,
    );
    reader.arm_liveness_for_test(root.identity).unwrap();
    drop(writer);
    let identity = root.identity;
    root.terminate();
    wait_for_release(&reader);
    let mut proof = PtySessionProof::capture_bound(master, TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();
    (proof, identity)
}

fn wait_for_release(reader: &crate::process::MacosProcessCapabilityManagerReader) {
    let deadline = Instant::now() + RELEASE_TIMEOUT;
    while reader.liveness_observation()
        != Some(MacosProcessCapabilityObservation::ReleasedAfterRootExit)
    {
        assert!(Instant::now() < deadline, "{RELEASE_TIMEOUT_DETAIL}");
        std::thread::sleep(RELEASE_POLL_INTERVAL);
    }
}

fn receive_right(receiver: &UnixStream) -> OwnedFd {
    crate::local_unix::macos::receive_exactly_one_right(receiver.as_raw_fd(), QUEUED_RIGHT_MARKER)
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

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
