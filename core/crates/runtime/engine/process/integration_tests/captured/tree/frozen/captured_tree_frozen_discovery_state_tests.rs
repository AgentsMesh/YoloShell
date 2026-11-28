use super::{FrozenClosureAuthority, stopped_closure};
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree_macos_wait::TEST_TIMEOUT;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess};
use crate::process::macos_process_capability::test_support;
use crate::process::{
    CapturedProcessTree, FreezingProcessTree, LiveProcessIdentity, MacosProcessCapabilityIssuer,
    ProcessIdentity, PtySessionProof,
};
use portable_pty::{PtySize, native_pty_system};
use std::time::Instant;

const TEST_CAPABILITY: &str = "frozen-refreeze-epoch";
const DISCOVERY_COVERAGE_TIMEOUT_DETAIL: &str = "writer-present discovery did not converge";
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;

#[test]
fn foreign_unresolved_source_is_never_acknowledged() {
    let root = TestProcess::spawn();
    let foreign = TestProcess::spawn();
    let mut tree = covered_tree_with_unresolved_source(root.identity, foreign.identity);
    let frozen_root = tree.live_root();

    assert!(!tree.acknowledge_frozen_forks(&[frozen_root]));
    assert!(tree.events.has_unresolved_fork());
}

#[test]
fn gone_or_identity_replaced_source_without_exit_proof_is_not_frozen() {
    let mut root = TestProcess::spawn();
    let mut tree = covered_tree_with_unresolved_source(root.identity, root.identity);
    root.terminate();
    let targets = tree.captured_targets();

    assert!(stopped_closure(&mut tree, targets).unwrap().is_none());
    assert!(tree.events.has_unresolved_fork());
}

#[test]
fn incomplete_coverage_and_pending_capability_both_reject_acknowledgement() {
    let root = TestProcess::spawn();
    let mut tree = covered_tree_with_unresolved_source(root.identity, root.identity);
    let frozen_sources = [tree.live_root()];
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    assert!(!tree.acknowledge_frozen_forks(&frozen_sources));
    tree.discovery_epoch = DiscoveryScanEpoch::Covered;
    tree.capability_scan_pending = true;
    assert!(!tree.acknowledge_frozen_forks(&frozen_sources));
    assert!(!tree.positive_closure_coverage(&frozen_sources));
    assert!(tree.events.has_unresolved_fork());
}

#[test]
fn writer_present_coverage_authorizes_only_the_frozen_closure() {
    let root = TestProcess::spawn();
    let (mut tree, _writer) = writer_present_tree(&root);
    let frozen_sources = [tree.live_root()];

    assert!(tree.capability_scan_pending);
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Covered);
    assert!(tree.positive_closure_coverage(&frozen_sources));
    assert!(!tree.frozen_closure_is_settled(FrozenClosureAuthority::OutputFencedEmpty));

    let frozen_root = frozen_sources[0];
    tree.events.inject_unresolved_fork(frozen_root);
    assert!(tree.acknowledge_frozen_forks(&[frozen_root]));
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert!(!tree.positive_closure_coverage(&frozen_sources));
}

#[test]
fn released_coverage_authorizes_only_the_positive_frozen_closure() {
    let root = TestProcess::spawn();
    let (mut tree, _writer) = writer_present_tree(&root);
    tree.pty_scan.mark_released_needs_scan_for_test();
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    let frozen_sources = [tree.live_root()];

    tree.capability_scheduling_complete = false;
    assert!(!tree.positive_closure_coverage(&frozen_sources));
    tree.capability_scheduling_complete = true;
    assert!(tree.capability_scheduling_complete);
    assert!(tree.positive_closure_coverage(&frozen_sources));
    assert!(!tree.frozen_closure_is_settled(FrozenClosureAuthority::OutputFencedEmpty));

    let frozen_root = frozen_sources[0];
    tree.events.inject_unresolved_fork(frozen_root);
    assert!(tree.acknowledge_frozen_forks(&[frozen_root]));
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert!(!tree.capability_scheduling_complete);
    assert!(!tree.positive_closure_coverage(&frozen_sources));
}

#[test]
fn pending_empty_closure_requires_released_capability_for_kernel_eof() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let root = TestProcess::spawn();
    let proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    tree.capability_scan_pending = true;
    tree.lineage_settled = true;
    tree.lineage_integrity_lost = false;

    assert!(!tree.frozen_closure_is_settled(FrozenClosureAuthority::OutputFencedEmpty));
    tree.pty_scan.mark_released_needs_scan_for_test();
    assert!(tree.frozen_closure_is_settled(FrozenClosureAuthority::OutputFencedEmpty));
    assert!(!tree.positive_closure_coverage(&[]));
}

#[test]
fn every_pty_refreeze_starts_a_fresh_discovery_epoch() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TERMINAL_ROWS,
            cols: TERMINAL_COLUMNS,
            pixel_width: TERMINAL_PIXEL_SIZE,
            pixel_height: TERMINAL_PIXEL_SIZE,
        })
        .unwrap();
    let root = TestProcess::spawn();
    let proof =
        PtySessionProof::capture(pair.master.tty_name().unwrap(), TEST_CAPABILITY.to_owned())
            .unwrap();
    let tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    let freezing = FreezingProcessTree::enter(tree, Default::default(), false);

    assert_eq!(freezing.tree.discovery_epoch, DiscoveryScanEpoch::Pending);
}

fn covered_tree_with_unresolved_source(
    root: ProcessIdentity,
    source: ProcessIdentity,
) -> CapturedProcessTree {
    let mut tree = CapturedProcessTree::capture(root).unwrap();
    let source = LiveProcessIdentity::capture(source).unwrap().unwrap();
    tree.events.inject_unresolved_fork(source);
    tree.discovery_epoch = DiscoveryScanEpoch::Covered;
    tree.lineage_settled = true;
    tree.lineage_integrity_lost = true;
    tree.capability_scan_pending = false;
    tree
}

pub(super) fn writer_present_tree(
    root: &TestProcess,
) -> (
    CapturedProcessTree,
    crate::process::MacosProcessCapabilityChildWriter,
) {
    let mut tree = CapturedProcessTree::capture_with_discovery_internal(
        root.identity,
        TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    tree.install_process_capability(reader).unwrap();
    tree.begin_discovery_cleanup_internal();
    tree.bind_repeating_process_inventory_for_test(&[(
        root.identity,
        NO_PARENT_PID,
        root.identity.pid,
    )]);
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !tree.discovery_epoch.coverage_is_complete() {
        assert!(tree.refresh_if_needed_internal().unwrap());
        assert!(
            Instant::now() < deadline,
            "{DISCOVERY_COVERAGE_TIMEOUT_DETAIL}: {:?}",
            tree.state_snapshot_for_test()
        );
        std::thread::yield_now();
    }
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Covered);
    (tree, writer)
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
