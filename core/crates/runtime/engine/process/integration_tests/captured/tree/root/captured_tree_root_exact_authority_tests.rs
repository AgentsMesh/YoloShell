use super::{FreezeClosureCandidate, exact_stopped_candidate};
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::inventory_snapshot::install_test_inventory;
use crate::process::inventory_stable::STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE;
use crate::process::inventory_test_support::{TestProcess, entry};
use crate::process::live_identity::ProcessExecutionState;
use crate::process::{
    CapturedProcessTree, MacosProcessCapabilityIssuer, PTY_SESSION_CAPABILITY_ENV, PtySessionProof,
};
use portable_pty::{PtySize, native_pty_system};
use std::sync::Arc;
use std::time::Instant;

const TEST_CAPABILITY: &str = "root-exact-capability-authority";
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const INVENTORY_REFRESH_BUDGET: usize = 16;
const ROOT_AUTHORITY_DETAIL: &str = "root exact authority did not bypass the ambient sweep";
const ROOT_MISMATCH_DETAIL: &str = "root descriptor mismatch produced exact authority";
const AMBIENT_COVERAGE_DETAIL: &str = "ambient coverage completed before root exact authority";
const STOP_TIMEOUT_DETAIL: &str = "capability root did not stop";

#[test]
fn exact_capability_root_bypasses_the_ambient_sweep() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    let mut root = TestProcess::spawn_with_environment_and_process_capability(
        PTY_SESSION_CAPABILITY_ENV,
        TEST_CAPABILITY,
        &writer,
    );
    let mut ambient = TestProcess::spawn();
    let inventory = vec![
        entry(root.identity, u32::default(), root.identity.pid),
        entry(ambient.identity, u32::default(), ambient.identity.pid),
    ];
    let _inventory = repeated_inventory(inventory);
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    proof
        .install_process_capability(Arc::clone(&reader))
        .unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    let root_live = tree.live_root();
    reader.verify_writer_holder(&root_live).unwrap();
    drop(writer);

    tree.begin_frozen_discovery();
    wait_for_root_authority(&mut tree, root_live);
    tree.terminate_captured(libc::SIGSTOP).unwrap();
    wait_for_stop(&tree);
    assert!(matches!(
        exact_stopped_candidate(&mut tree).unwrap(),
        Some(FreezeClosureCandidate::IncrementalExact(closure)) if closure == [root_live]
    ));
    root.terminate();
    ambient.terminate();
}

#[test]
fn root_without_the_exact_descriptor_remains_unauthorized() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    let mut root = TestProcess::spawn_with_environment(PTY_SESSION_CAPABILITY_ENV, TEST_CAPABILITY);
    let inventory = vec![entry(root.identity, u32::default(), root.identity.pid)];
    let _inventory = repeated_inventory(inventory);
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    let root_live = tree.live_root();

    tree.begin_frozen_discovery();
    tree.refresh().unwrap();
    assert!(
        !tree.pty_scan.covers_exact_released_closure(&[root_live]),
        "{ROOT_MISMATCH_DETAIL}"
    );
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    tree.terminate_captured(libc::SIGSTOP).unwrap();
    wait_for_stop(&tree);
    let targets = tree.captured_targets();
    let authority = tree
        .captured_stopped_closure_authority(&targets)
        .unwrap()
        .unwrap();
    assert!(
        !tree.incremental_stopped_closure_can_kill(&authority),
        "{ROOT_MISMATCH_DETAIL}"
    );
    drop(writer);
    root.terminate();
}

fn wait_for_root_authority(
    tree: &mut CapturedProcessTree,
    root: crate::process::LiveProcessIdentity,
) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        assert!(
            !tree.discovery_epoch.coverage_is_complete(),
            "{AMBIENT_COVERAGE_DETAIL}: {:#?}",
            tree.state_snapshot_for_test()
        );
        tree.refresh().unwrap();
        if tree.pty_scan.covers_exact_released_closure(&[root]) {
            assert!(!tree.discovery_epoch.coverage_is_complete());
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{ROOT_AUTHORITY_DETAIL}: {:#?}",
            tree.state_snapshot_for_test()
        );
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn repeated_inventory(
    inventory: Vec<crate::process::inventory_snapshot::InventoryEntry>,
) -> crate::process::inventory_snapshot::TestInventoryGuard {
    install_test_inventory(std::iter::repeat_n(
        inventory,
        STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE * INVENTORY_REFRESH_BUDGET,
    ))
}

fn wait_for_stop(tree: &CapturedProcessTree) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while tree.captured_targets().iter().any(|(identity, _durable)| {
        identity.execution_state().unwrap() != ProcessExecutionState::Stopped
    }) {
        assert!(Instant::now() < deadline, "{STOP_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
