use super::*;
use crate::process::inventory_test_support::entry;
use crate::process::{LiveProcessIdentity, MacosProcessCapabilityIssuer, ProcessIdentity};
use fork_exec_support::ForkExecChild;
use portable_pty::{PtySize, native_pty_system};

#[path = "../pty_session_fork_exec_support_macos.rs"]
mod fork_exec_support;
#[path = "../../../../test_support/pty_session/pty_session_spawn_fence_isolation.rs"]
mod isolation;

const INNER_TEST: &str = "pty_session_proof::macos_scan::\
spawn_fence_process_tests::fork_exec_scan_helper";
const INNER_ROLE_ENV: &str = "YOLO_FORK_EXEC_SCAN_ROLE";
const INNER_COMPLETION_ENV: &str = "YOLO_FORK_EXEC_SCAN_COMPLETION";
const INNER_ROLE: &str = "fork-exec-scan";
const EMPTY_MARKER: &[u8] = b"";
const TEST_CAPABILITY: &str = "fork-exec-descriptor-scan";
const TEST_SESSION_ID: u32 = 93_001;
const ABSENT_ROOT_LINEAGE: u64 = u64::MAX;
const ROOT_EXITED: bool = false;
const SCAN_ESCAPED_PROCESSES: bool = false;
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const FCNTL_FAILURE: libc::c_int = -1;

#[test]
fn fork_exec_cloexec_inheritance_never_becomes_owned_scan_evidence() {
    isolation::run_isolated(INNER_TEST, INNER_ROLE_ENV, INNER_ROLE, INNER_COMPLETION_ENV);
}

#[test]
fn fork_exec_scan_helper() {
    if std::env::var(INNER_ROLE_ENV).as_deref() != Ok(INNER_ROLE) {
        return;
    }
    run_process_proof();
    std::fs::write(
        std::env::var_os(INNER_COMPLETION_ENV).unwrap(),
        EMPTY_MARKER,
    )
    .unwrap();
}

fn run_process_proof() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TERMINAL_ROWS,
            cols: TERMINAL_COLUMNS,
            pixel_width: TERMINAL_PIXEL_SIZE,
            pixel_height: TERMINAL_PIXEL_SIZE,
        })
        .unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    assert_cloexec(writer.descriptor());
    let mut escaped = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    escaped
        .install_process_capability(std::sync::Arc::clone(&reader))
        .unwrap();
    let mut available =
        PtySessionProof::capture(pair.master.tty_name().unwrap(), TEST_CAPABILITY.to_owned())
            .unwrap();
    available
        .install_process_capability(std::sync::Arc::clone(&reader))
        .unwrap();

    let spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    let mut child = ForkExecChild::spawn(spawn_guard.boundary());
    let identity = ProcessIdentity::capture(child.pid()).unwrap();
    let live = LiveProcessIdentity::capture(identity).unwrap().unwrap();
    reader.verify_writer_holder(&live).unwrap();
    let inventory = inventory_for(identity);

    let escaped_pending = scan(&inventory, &escaped, identity);
    assert_empty_pending(&escaped_pending);
    let available_pending = scan(&inventory, &available, identity);
    assert_empty_pending(&available_pending);

    child.release_for_exec();
    child.wait_for_exec();
    child.assert_running();
    drop(spawn_guard);
    assert!(reader.verify_writer_holder(&live).is_err());

    let fresh = scan(&inventory, &escaped, identity);
    assert!(fresh.candidates.observed.is_empty());
    assert!(fresh.candidates.stable.is_empty());
    assert_eq!(fresh.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(fresh.coverage_status, CapabilityScanStatus::Complete);
    child.assert_running();

    drop(writer);
    child.assert_running();
}

fn scan(
    inventory: &StableInventory,
    proof: &PtySessionProof,
    root: ProcessIdentity,
) -> CapabilityScanResult {
    inventory
        .macos_capability_candidates(
            proof,
            &PtyCapabilityScanState::default(),
            &Default::default(),
            &Default::default(),
            root.start_time,
            ABSENT_ROOT_LINEAGE,
            ROOT_EXITED,
            SCAN_ESCAPED_PROCESSES,
        )
        .unwrap()
}

fn assert_empty_pending(scan: &CapabilityScanResult) {
    assert!(scan.candidates.observed.is_empty());
    assert!(scan.candidates.stable.is_empty());
    assert_eq!(scan.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(scan.coverage_status, CapabilityScanStatus::Pending);
}

fn inventory_for(identity: ProcessIdentity) -> StableInventory {
    let value = entry(identity, std::process::id(), TEST_SESSION_ID);
    let lineage = value.lineage_id();
    StableInventory {
        entries: vec![value],
        observed: vec![value],
        by_pid: HashMap::from([(value.pid, value)]),
        by_live: HashMap::from([(value.live_identity().unwrap(), value)]),
        trusted_parents: HashMap::new(),
        possible_parents: HashMap::new(),
        observed_sessions: HashMap::new(),
        observed_lineages: HashSet::from([lineage]),
        stable_lineages: HashSet::from([lineage]),
        unstable_lineages: HashSet::new(),
    }
}

fn assert_cloexec(descriptor: libc::c_int) {
    // SAFETY: descriptor is the live capability writer owned by this scope.
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
}
