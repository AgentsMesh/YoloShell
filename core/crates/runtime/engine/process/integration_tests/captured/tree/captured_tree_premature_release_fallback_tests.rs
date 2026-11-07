use super::stopped_closure;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::inventory_snapshot::install_repeating_test_inventory;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use crate::process::live_identity::ProcessExecutionState;
use crate::process::{
    CapturedProcessTree, LiveProcessIdentity, MacosProcessCapabilityIssuer,
    MacosProcessCapabilityObservation, PtySessionProof,
};
use portable_pty::{PtySize, native_pty_system};
use std::sync::Arc;
use std::time::Instant;

const TEST_CAPABILITY: &str = "confirmed-bootstrap-tree-fallback";
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const COVERAGE_TIMEOUT_DETAIL: &str = "premature-release fallback coverage did not settle";
const LIVENESS_TIMEOUT_DETAIL: &str = "premature-release observation did not settle";
const ROOT_IDENTITY_QUERY_FAILURE_DETAIL: &str = "root identity query unavailable";

#[test]
fn confirmed_bootstrap_release_requires_acknowledgement_and_a_fresh_scan() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let mut root = TestProcess::spawn();
    let root_live = LiveProcessIdentity::capture(root.identity)
        .unwrap()
        .unwrap();
    let mut proof =
        PtySessionProof::capture_bootstrap_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned())
            .unwrap();
    proof
        .take_bootstrap_exec_authority()
        .unwrap()
        .confirm_exec()
        .unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    reader.arm_liveness(root_live.clone()).unwrap();
    proof
        .install_process_capability(Arc::clone(&reader))
        .unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    tree.authorize_destructive_for_test();
    drop(writer);
    wait_for_release(&reader);

    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let _inventory = install_repeating_test_inventory(vec![root_entry]);
    tree.begin_frozen_discovery();
    wait_for_coverage(&mut tree);
    assert!(tree.capability_scan_pending);
    assert!(tree.pty_scan.uses_premature_release_fallback());

    tree.terminate_captured(libc::SIGSTOP).unwrap();
    wait_for_stopped(&root_live);
    tree.events.queue_test_fork(root_live.clone());
    let targets = tree.captured_targets();
    assert!(stopped_closure(&mut tree, targets).unwrap().is_none());
    assert!(!tree.events.has_unresolved_fork());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);

    wait_for_coverage(&mut tree);
    let targets = tree.captured_targets();
    let closure = stopped_closure(&mut tree, targets)
        .unwrap()
        .expect("fresh fallback scan did not authorize the frozen closure");
    assert_eq!(closure, [root_live]);
    root.terminate();
}

#[test]
fn root_exit_after_fork_acknowledgement_authorizes_only_a_fresh_empty_closure() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let mut root = TestProcess::spawn();
    let root_live = LiveProcessIdentity::capture(root.identity)
        .unwrap()
        .unwrap();
    let mut proof =
        PtySessionProof::capture_bootstrap_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned())
            .unwrap();
    proof
        .take_bootstrap_exec_authority()
        .unwrap()
        .confirm_exec()
        .unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    reader.arm_liveness(root_live.clone()).unwrap();
    proof
        .install_process_capability(Arc::clone(&reader))
        .unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    tree.authorize_destructive_for_test();
    drop(writer);
    wait_for_release(&reader);

    let _inventory = install_repeating_test_inventory(vec![entry(
        root.identity,
        NO_PARENT_PID,
        root.identity.pid,
    )]);
    tree.begin_frozen_discovery();
    wait_for_coverage(&mut tree);
    assert!(!tree.empty_output_fence_is_authorized());
    tree.terminate_captured(libc::SIGSTOP).unwrap();
    wait_for_stopped(&root_live);
    tree.events.queue_test_fork(root_live.clone());
    let targets = tree.captured_targets();
    assert!(stopped_closure(&mut tree, targets).unwrap().is_none());
    wait_for_coverage(&mut tree);
    let targets = tree.captured_targets();
    let closure = stopped_closure(&mut tree, targets)
        .unwrap()
        .expect("fresh fallback scan did not authorize the frozen root");
    assert_eq!(closure, [root_live]);

    root.terminate();
    tree.mark_root_exited_unreaped(tree.root_exit_proof_for_test(root.identity))
        .unwrap();
    assert!(!tree.empty_output_fence_is_authorized());
    wait_for_empty_closure(&mut tree);
    let _failure = crate::process::install_test_match_failure(ROOT_IDENTITY_QUERY_FAILURE_DETAIL);
    assert!(tree.empty_output_fence_is_authorized());
}

fn wait_for_release(reader: &crate::process::MacosProcessCapabilityManagerReader) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while reader.liveness_observation()
        != Some(MacosProcessCapabilityObservation::PrematurelyReleased)
    {
        assert!(Instant::now() < deadline, "{LIVENESS_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_coverage(tree: &mut CapturedProcessTree) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !tree.discovery_epoch.coverage_is_complete() {
        tree.refresh().unwrap();
        assert!(Instant::now() < deadline, "{COVERAGE_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_stopped(identity: &LiveProcessIdentity) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while identity.execution_state().unwrap() != ProcessExecutionState::Stopped {
        assert!(Instant::now() < deadline, "{COVERAGE_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_empty_closure(tree: &mut CapturedProcessTree) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        tree.refresh().unwrap();
        let targets = tree.captured_targets();
        if stopped_closure(tree, targets)
            .unwrap()
            .is_some_and(|closure| closure.is_empty())
        {
            return;
        }
        assert!(Instant::now() < deadline, "{COVERAGE_TIMEOUT_DETAIL}");
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
