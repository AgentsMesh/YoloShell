use super::*;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
#[cfg(target_os = "macos")]
use crate::process::captured_tree_discovery_test_support::DiscoveryTreeTestFixture;
#[cfg(target_os = "macos")]
use crate::process::constants::EVENT_RECONCILIATION_LIMIT;
#[cfg(target_os = "macos")]
use crate::process::inventory_snapshot::install_test_inventory;
#[cfg(target_os = "macos")]
use crate::process::inventory_stable::STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE;
use crate::process::inventory_test_support::TestProcess;

const UNEXPECTED_PUBLICATION_AUDIT_RESTART: &str =
    "completed dense publication audit was restarted";
const SECOND_SCAN_TEST_CAPABILITY: &str = "dense-publication-capability";
const REGRESSION_TEST_CAPABILITY: &str = "dense-publication-regression-capability";

#[test]
fn publication_requires_an_authoritative_empty_epoch() {
    let root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();

    assert!(!tree.discovery_publication_ready_internal().unwrap());
}

#[test]
fn publication_restarts_when_an_observed_target_is_still_live() {
    let root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    mark_authoritative_empty(&mut tree);

    assert!(!tree.discovery_publication_ready_internal().unwrap());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
}

#[test]
fn publication_accepts_an_empty_epoch_after_the_final_event_drain() {
    let mut root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    root.terminate();
    tree.mark_root_exited_unreaped(tree.root_exit_proof_for_test(root.identity))
        .unwrap();
    tree.captured.clear();
    settle_event_publication(&mut tree);
    mark_authoritative_empty(&mut tree);

    assert!(tree.discovery_publication_ready_internal().unwrap());
}

#[cfg(target_os = "macos")]
#[test]
fn publication_rejects_a_final_event_invalidation() {
    let mut root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    root.terminate();
    tree.mark_root_exited_unreaped(tree.root_exit_proof_for_test(root.identity))
        .unwrap();
    tree.captured.clear();
    settle_event_publication(&mut tree);
    tree.events.invalidate();
    mark_authoritative_empty(&mut tree);

    assert!(!tree.discovery_publication_ready_internal().unwrap());
}

#[cfg(target_os = "macos")]
#[test]
fn publication_uses_a_second_authoritative_scan_after_gate_loss() {
    let mut fixture = DiscoveryTreeTestFixture::spawn(SECOND_SCAN_TEST_CAPABILITY);
    fixture.exit_root();
    fixture.wait_for_kernel_eof();
    refresh_empty(&mut fixture.tree);
    fixture.tree.captured.clear();
    fixture.tree.fail_next_event_read();
    assert!(fixture.tree.events.requires_reconciliation());
    mark_authoritative_empty(&mut fixture.tree);
    let _inventory = install_test_inventory(std::iter::repeat_n(
        Vec::new(),
        EVENT_RECONCILIATION_LIMIT * STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE,
    ));

    assert!(fixture.tree.discovery_publication_ready_internal().unwrap());
    assert!(!fixture.tree.events.is_gated());
}

#[cfg(target_os = "macos")]
#[test]
fn completed_dense_publication_audit_is_not_restarted_between_watchdog_polls() {
    let mut root = TestProcess::spawn();
    let mut tree = ungated_empty_discovery_tree(&mut root);
    tree.begin_dense_publication_audit();
    assert_eq!(
        tree.discovery_publication_audit,
        DiscoveryPublicationAudit::InProgress
    );
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);

    tree.discovery_epoch = DiscoveryScanEpoch::Covered;
    tree.discovery_authoritative_empty = true;
    tree.lineage_settled = true;
    tree.capability_scan_pending = true;
    assert!(!tree.discovery_publication_ready_internal().unwrap());
    assert_eq!(
        tree.discovery_publication_audit,
        DiscoveryPublicationAudit::InProgress
    );

    mark_authoritative_empty(&mut tree);
    tree.fail_next_refresh(UNEXPECTED_PUBLICATION_AUDIT_RESTART);

    assert!(tree.discovery_publication_ready_internal().unwrap());
    assert_eq!(
        tree.discovery_publication_audit,
        DiscoveryPublicationAudit::InProgress
    );
}

#[cfg(target_os = "macos")]
#[test]
fn new_discovery_cleanup_invalidates_an_in_progress_publication_audit() {
    let mut root = TestProcess::spawn();
    let mut tree = ungated_empty_discovery_tree(&mut root);
    tree.begin_dense_publication_audit();

    tree.begin_discovery_cleanup_internal();

    assert_eq!(
        tree.discovery_publication_audit,
        DiscoveryPublicationAudit::Required
    );
}

#[cfg(target_os = "macos")]
fn ungated_empty_discovery_tree(root: &mut TestProcess) -> CapturedProcessTree {
    let mut tree = CapturedProcessTree::capture_with_discovery_internal(
        root.identity,
        REGRESSION_TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    root.terminate();
    tree.mark_root_exited_unreaped(tree.root_exit_proof_for_test(root.identity))
        .unwrap();
    tree.captured.clear();
    settle_event_publication(&mut tree);
    tree.fail_next_event_read();
    assert!(tree.events.requires_reconciliation());
    assert!(!tree.events.is_gated());
    mark_authoritative_empty(&mut tree);
    tree
}

fn mark_authoritative_empty(tree: &mut CapturedProcessTree) {
    tree.discovery_epoch = DiscoveryScanEpoch::Complete;
    tree.discovery_authoritative_empty = true;
    tree.lineage_settled = true;
    tree.lineage_integrity_lost = false;
    tree.capability_scan_pending = false;
    tree.retiring.clear();
}

fn settle_event_publication(tree: &mut CapturedProcessTree) {
    let _ = tree.events.reconcile_inventory(std::iter::empty());
    assert!(tree.events.requires_reconciliation());
    assert!(tree.events.reconcile_inventory(std::iter::empty()));
}

#[cfg(target_os = "macos")]
fn refresh_empty(tree: &mut CapturedProcessTree) {
    let _inventory = install_test_inventory([Vec::new(), Vec::new()]);
    tree.refresh().unwrap();
}
