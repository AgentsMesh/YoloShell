use super::FrozenClosureAuthority;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::inventory_test_support::TestProcess;
use crate::process::live_identity::ProcessExecutionState;
use crate::process::{CapturedProcessTree, DescendantObservation};
use std::time::Instant;

const REPLACEMENT_GENERATION_STEP: u64 = 1;
const STOP_TIMEOUT_DETAIL: &str = "captured batch did not stop";

#[test]
fn pending_discovery_freezes_only_an_exact_authorized_nonempty_batch() {
    let mut root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    tree.terminate_captured(libc::SIGSTOP).unwrap();
    wait_for_stop(&tree);
    tree.lineage_settled = true;
    tree.lineage_integrity_lost = false;
    tree.capability_scan_pending = true;
    assert_eq!(
        tree.descendant_observation(),
        DescendantObservation::CapabilityScanPending
    );
    let targets = tree.captured_targets();

    tree.deny_destructive_for_test();
    assert!(tree.captured_stopped_closure_authority(&targets).is_err());
    tree.authorize_destructive_for_test();
    let mut replaced = targets.clone();
    let replacement = replaced[0]
        .0
        .unique_id()
        .checked_add(REPLACEMENT_GENERATION_STEP)
        .unwrap();
    replaced[0].0 = replaced[0].0.with_generation(replacement);
    assert!(
        tree.captured_stopped_closure_authority(&replaced)
            .unwrap()
            .is_none()
    );

    let authority = tree
        .captured_stopped_closure_authority(&targets)
        .unwrap()
        .unwrap();
    assert!(tree.frozen_closure_is_settled(FrozenClosureAuthority::CapturedStopped(&authority)));
    assert!(!tree.frozen_closure_is_settled(FrozenClosureAuthority::OutputFencedEmpty));

    tree.capability_scan_pending = false;
    tree.captured.clear();
    assert_eq!(
        tree.descendant_observation(),
        DescendantObservation::Settled
    );
    assert!(!tree.frozen_closure_is_settled(FrozenClosureAuthority::CapturedStopped(&authority)));

    root.terminate();
}

fn wait_for_stop(tree: &CapturedProcessTree) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while tree.live_root().execution_state().unwrap() != ProcessExecutionState::Stopped {
        assert!(Instant::now() < deadline, "{STOP_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}
