use super::exact_gate_authority_tests::wait_for_stop;
use super::frozen_discovery_state_tests::writer_present_tree;
use super::{exact_stopped_candidate, observe};
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::inventory_test_support::TestProcess;

#[test]
fn covered_fork_is_acknowledged_before_the_next_partial_sweep() {
    let mut root = TestProcess::spawn();
    let (mut tree, _writer) = writer_present_tree(&root);
    tree.authorize_destructive_for_test();
    tree.terminate_captured(libc::SIGSTOP).unwrap();
    wait_for_stop(&tree);
    let source = tree.live_root();
    tree.events.inject_unresolved_fork(source);
    tree.lineage_integrity_lost = true;

    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Covered);
    assert!(exact_stopped_candidate(&mut tree).unwrap().is_none());
    let mut destructive_started = false;

    assert!(
        observe(&mut tree, &mut destructive_started)
            .unwrap()
            .is_none()
    );
    assert!(!destructive_started);
    assert!(!tree.events.has_unresolved_fork());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert!(tree.capability_scan_pending);
    assert!(!tree.capability_scheduling_complete);
    assert!(tree.frozen_fork_reconciliation_required);

    root.terminate();
}
