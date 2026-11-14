use super::*;
use crate::process::ProcessIdentity;
use crate::process::captured_lineage::LineageObservationStatus;
use crate::process::captured_tree::discovery::{DiscoveryEpochProgress, DiscoveryScanEpoch};
use crate::process::inventory_test_support::TestProcess;
use std::collections::HashSet;

const TEST_CAPABILITY: &str = "root-exit-discovery-capability";

fn proof_tree(root: ProcessIdentity) -> CapturedProcessTree {
    CapturedProcessTree::capture_with_discovery_internal(root, TEST_CAPABILITY.to_owned()).unwrap()
}

fn compact_complete(tree: &mut CapturedProcessTree) {
    tree.compact(
        &HashSet::new(),
        &HashSet::new(),
        LineageObservationStatus::Complete,
        false,
        true,
        true,
        &[],
    )
    .unwrap();
}

#[test]
fn first_root_exit_restarts_exactly_one_discovery_epoch() {
    let root = TestProcess::spawn();
    let mut tree = proof_tree(root.identity);
    tree.discovery_epoch = DiscoveryScanEpoch::Complete;
    let proof = tree.root_exit_proof_for_test(root.identity);

    tree.mark_root_exited_unreaped(proof).unwrap();
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);

    tree.discovery_epoch = DiscoveryScanEpoch::Complete;
    tree.mark_root_exited_unreaped(proof).unwrap();
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Complete);
}

#[test]
fn authoritative_epoch_acknowledges_only_the_preexisting_fork_uncertainty() {
    let root = TestProcess::spawn();
    let mut tree = proof_tree(root.identity);
    let live_root = tree.live_root();
    tree.mark_root_exited_unreaped(tree.root_exit_proof_for_test(root.identity))
        .unwrap();
    tree.events.inject_unresolved_fork(live_root);

    compact_complete(&mut tree);

    assert!(!tree.lineage_integrity_lost);
    assert!(!tree.captured.is_empty());
    tree.discovery_epoch
        .observe(DiscoveryEpochProgress::covered(false, true));
    tree.events.inject_unresolved_fork(live_root);
    compact_complete(&mut tree);
    assert!(tree.lineage_integrity_lost);
}

#[test]
fn pending_capability_scan_preserves_root_fork_uncertainty() {
    let mut root = TestProcess::spawn();
    let mut tree = proof_tree(root.identity);
    let live_root = tree.live_root();
    root.terminate();
    tree.mark_root_exited_unreaped(tree.root_exit_proof_for_test(root.identity))
        .unwrap();
    tree.events.inject_unresolved_fork(live_root);

    tree.compact(
        &HashSet::new(),
        &HashSet::new(),
        LineageObservationStatus::Complete,
        true,
        true,
        true,
        &[],
    )
    .unwrap();

    assert!(tree.events.has_unresolved_fork());
    assert!(tree.lineage_integrity_lost);
}

#[test]
fn no_proof_tree_cannot_acknowledge_uncertainty_from_an_empty_scan() {
    let root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let live_root = tree.live_root();
    tree.begin_discovery_cleanup_internal();
    tree.events.inject_unresolved_fork(live_root);

    compact_complete(&mut tree);

    assert!(tree.lineage_integrity_lost);
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
}
