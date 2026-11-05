use super::stopped_closure;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree::frozen_discovery::FrozenForkCoverage;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::inventory_snapshot::install_test_inventory;
use crate::process::inventory_stable::STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use crate::process::{
    CapturedProcessTree, LiveProcessIdentity, MacosProcessCapabilityIssuer,
    PTY_SESSION_CAPABILITY_ENV, PtySessionProof,
};
use portable_pty::{PtySize, native_pty_system};
use std::sync::Arc;
use std::time::Instant;

const TEST_CAPABILITY: &str = "frozen-exact-released-authority";
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const INVENTORY_CAPTURE_BUDGET: usize = 8;
const EXACT_MATCH_TIMEOUT_DETAIL: &str = "released exact match was not retained";
const STOP_TIMEOUT_DETAIL: &str = "captured exact closure did not stop";
const RECONCILIATION_TIMEOUT_DETAIL: &str =
    "fresh inventory and event reconciliation did not authorize the captured closure";

#[test]
fn exact_source_fork_requires_fresh_reconciliation_before_freeze() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    let mut root = TestProcess::spawn_with_environment_and_process_capability(
        PTY_SESSION_CAPABILITY_ENV,
        TEST_CAPABILITY,
        &writer,
    );
    let mut escaped = TestProcess::spawn_with_environment_and_process_capability(
        PTY_SESSION_CAPABILITY_ENV,
        TEST_CAPABILITY,
        &writer,
    );
    let escaped_live = LiveProcessIdentity::capture(escaped.identity)
        .unwrap()
        .unwrap();
    drop(writer);
    let inventory = vec![
        entry(root.identity, NO_PARENT_PID, root.identity.pid),
        entry(escaped.identity, NO_PARENT_PID, escaped.identity.pid),
    ];
    let _inventory = install_test_inventory(std::iter::repeat_n(
        inventory,
        STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE * INVENTORY_CAPTURE_BUDGET,
    ));
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    proof
        .install_process_capability(Arc::clone(&reader))
        .unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    tree.begin_frozen_discovery();
    wait_for_exact_match(&mut tree, &escaped_live);
    tree.pty_scan.mark_released_needs_scan_for_test();
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    tree.events.inject_unresolved_fork(escaped_live);

    assert!(tree.positive_closure_coverage(&[escaped_live]));
    assert!(tree.acknowledge_frozen_forks(&[escaped_live]));
    assert!(!tree.events.has_unresolved_fork());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert!(tree.frozen_fork_reconciliation_required);
    assert!(!tree.positive_closure_coverage(&[escaped_live]));
    assert!(tree.frozen_fork_acknowledgement_coverage(&[escaped_live]));
    tree.observe_frozen_fork_reconciliation(FrozenForkCoverage::MISSING_CAPABILITY);
    assert!(tree.frozen_fork_reconciliation_required);

    tree.terminate_captured(libc::SIGSTOP).unwrap();
    wait_for_stopped_closure(&tree);
    let targets = tree.captured_targets();
    assert!(
        stopped_closure(&mut tree, targets.clone())
            .unwrap()
            .is_none()
    );
    assert!(stopped_closure(&mut tree, targets).unwrap().is_none());

    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        tree.refresh().unwrap();
        let targets = tree.captured_targets();
        if stopped_closure(&mut tree, targets).unwrap().is_some() {
            break;
        }
        assert!(Instant::now() < deadline, "{RECONCILIATION_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
    assert!(!tree.frozen_fork_reconciliation_required);

    root.terminate();
    escaped.terminate();
}

fn wait_for_stopped_closure(tree: &CapturedProcessTree) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while tree.captured_targets().iter().any(|(identity, _durable)| {
        identity.execution_state().unwrap()
            != crate::process::live_identity::ProcessExecutionState::Stopped
    }) {
        assert!(Instant::now() < deadline, "{STOP_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_exact_match(tree: &mut CapturedProcessTree, identity: &LiveProcessIdentity) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !tree.pty_scan.covers_exact_released_closure(&[*identity]) {
        tree.refresh().unwrap();
        assert!(Instant::now() < deadline, "{EXACT_MATCH_TIMEOUT_DETAIL}");
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
