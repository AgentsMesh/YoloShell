use super::*;
use crate::process::LiveProcessIdentity;
use crate::process::captured_lineage::LineageObservationStatus;
use crate::process::inventory_test_support::TestProcess;
use std::collections::HashSet;

#[cfg(target_os = "macos")]
const REFRESH_BRANCH_CAPABILITY: &str = "refresh-branch-discovery";

#[test]
fn descendant_observation_covers_pending_integrity_and_retiring_states() {
    let root = TestProcess::spawn();
    let child = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    tree.lineage_settled = true;
    tree.lineage_integrity_lost = false;
    tree.retiring.clear();

    let live_root = tree.scope.live_root();
    tree.retiring.insert(live_root);
    tree.capability_scan_pending = true;
    assert_eq!(
        tree.descendant_observation(),
        DescendantObservation::CapabilityScanPending
    );

    tree.capability_scan_pending = false;
    tree.retiring.clear();
    tree.lineage_integrity_lost = true;
    assert_eq!(
        tree.descendant_observation(),
        DescendantObservation::Indeterminate
    );

    tree.lineage_integrity_lost = false;
    let live_child = LiveProcessIdentity::capture(child.identity)
        .unwrap()
        .unwrap();
    tree.retiring.insert(live_child);
    assert_eq!(
        tree.descendant_observation(),
        DescendantObservation::Indeterminate
    );
}

#[test]
fn kernel_eof_deferral_without_capability_proof_completes_scan() {
    let root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    tree.capability_scan_pending = true;

    tree.refresh_for_kernel_eof().unwrap();

    assert!(!tree.capability_scan_pending);
}

#[cfg(target_os = "macos")]
#[test]
fn completed_empty_discovery_supersedes_historical_lineage_uncertainty() {
    let mut root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture_with_discovery_internal(
        root.identity,
        REFRESH_BRANCH_CAPABILITY.to_owned(),
    )
    .unwrap();
    tree.captured.clear();
    tree.retiring.clear();
    tree.lineage_integrity_lost = true;
    root.terminate();
    let proof = tree.root_exit_proof_for_test(root.identity);
    tree.mark_root_exited_unreaped(proof).unwrap();
    tree.events.invalidate();

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

    assert!(tree.lineage_integrity_lost);
    assert!(tree.events.requires_reconciliation());
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

    assert!(!tree.lineage_integrity_lost);
}
