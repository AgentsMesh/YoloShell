use super::{
    FreezeClosureCandidate, exact_stopped_candidate, exact_stopped_closure, stopped_closure,
};
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::inventory_snapshot::install_repeating_test_inventory;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use crate::process::live_identity::ProcessExecutionState;
use crate::process::{
    CapturedProcessTree, DescendantObservation, LiveProcessIdentity, MacosProcessCapabilityIssuer,
    PTY_SESSION_CAPABILITY_ENV, PtySessionProof,
};
use portable_pty::{PtySize, native_pty_system};
use std::sync::Arc;
use std::time::Instant;

const TEST_CAPABILITY: &str = "incremental-kill-authority";
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const AUTHORITY_TIMEOUT_DETAIL: &str = "exact stopped closure authority was not established";

#[test]
fn exact_stopped_holder_is_killable_without_consuming_unresolved_root_fork() {
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
    let _inventory = install_repeating_test_inventory(inventory);
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    proof
        .install_process_capability(Arc::clone(&reader))
        .unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    let root_live = tree.live_root();
    tree.begin_frozen_discovery();
    tree.events.inject_unresolved_fork(root_live);
    root.terminate();
    wait_for_exact_single_target(&mut tree, escaped_live);
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    tree.terminate_captured(libc::SIGSTOP).unwrap();
    wait_for_stop(&tree);
    let candidate = exact_stopped_candidate(&mut tree).unwrap();
    assert!(
        matches!(
            candidate,
            Some(FreezeClosureCandidate::IncrementalExact(closure)) if closure == [escaped_live]
        ),
        "unexpected exact stopped candidate: {:#?}",
        tree.state_snapshot_for_test()
    );
    assert!(!tree.discovery_epoch.coverage_is_complete());
    assert!(tree.events.has_unresolved_fork());
    tree.refresh().unwrap();
    wait_for_covered_discovery(&mut tree);
    let closure = exact_stopped_closure(&mut tree).unwrap().unwrap();

    assert_eq!(closure, [escaped_live]);
    assert!(tree.events.has_unresolved_fork());
    assert!(tree.lineage_integrity_lost);
    assert_eq!(
        tree.descendant_observation(),
        DescendantObservation::Indeterminate
    );
    escaped.terminate();
}

#[test]
fn unresolved_unrelated_fork_without_descriptor_proof_remains_fail_closed() {
    let mut root = TestProcess::spawn();
    let mut unrelated = TestProcess::spawn();
    let unrelated_live = LiveProcessIdentity::capture(unrelated.identity)
        .unwrap()
        .unwrap();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    tree.terminate_captured(libc::SIGSTOP).unwrap();
    wait_for_stop(&tree);
    tree.discovery_epoch = DiscoveryScanEpoch::Covered;
    tree.lineage_settled = true;
    tree.events.inject_unresolved_fork(unrelated_live);
    let targets = tree.captured_targets();

    assert!(stopped_closure(&mut tree, targets).unwrap().is_none());
    assert!(tree.events.has_unresolved_fork());
    root.terminate();
    unrelated.terminate();
}

fn wait_for_exact_single_target(tree: &mut CapturedProcessTree, expected: LiveProcessIdentity) {
    wait_until(tree, |tree| {
        tree.captured_targets()
            .iter()
            .map(|(identity, _durable)| *identity)
            .eq([expected])
            && tree.pty_scan.covers_exact_released_closure(&[expected])
    });
}

fn wait_for_covered_discovery(tree: &mut CapturedProcessTree) {
    wait_until(tree, |tree| tree.discovery_epoch.coverage_is_complete());
}

fn wait_until(tree: &mut CapturedProcessTree, condition: impl Fn(&CapturedProcessTree) -> bool) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !condition(tree) {
        tree.refresh().unwrap();
        assert!(Instant::now() < deadline, "{AUTHORITY_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_stop(tree: &CapturedProcessTree) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while tree.captured_targets().iter().any(|(identity, _durable)| {
        identity.execution_state().unwrap() != ProcessExecutionState::Stopped
    }) {
        assert!(Instant::now() < deadline, "{AUTHORITY_TIMEOUT_DETAIL}");
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
