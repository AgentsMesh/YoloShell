use super::exact_gate_fixture::ExactGateFixture;
use super::{FreezeClosureCandidate, exact_stopped_candidate};
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree::frozen_discovery::FrozenForkCoverage;
use crate::process::{CapturedProcessTree, LiveProcessIdentity, ProcessIdentity};

const INCOMPLETE_COVERAGE_CAPABILITY: &str = "kernel-eof-incomplete-fork-debt";
const POST_INVENTORY_FORK_CAPABILITY: &str = "kernel-eof-post-inventory-fork-debt";
const KERNEL_ONLY_CAPABILITY: &str = "kernel-eof-without-exact-positive-authority";
const DUAL_AUTHORITY_CAPABILITY: &str = "incremental-precedes-kernel-eof-authority";
const LINEAGE_GAP_CAPABILITY: &str = "kernel-eof-production-lineage-gap";
const CAPABILITY_GAP_CAPABILITY: &str = "kernel-eof-production-capability-gap";
const PUBLICATION_GAP_CAPABILITY: &str = "kernel-eof-production-publication-gap";
const FRESH_COVERAGE_PRECONDITION_DETAIL: &str = "fresh coverage settlement was not required";
const FRESH_COVERAGE_INVARIANT_DETAIL: &str = "fresh coverage cleared debt without full authority";
const FROZEN_FORK_ACKNOWLEDGEMENT_DETAIL: &str = "frozen fork was not acknowledged";
const PARTIAL_COVERAGE_DEBT_DETAIL: &str = "partial coverage incorrectly cleared fork debt";
const STOPPED_CLOSURE_AUTHORITY_DETAIL: &str = "stopped closure authority is unavailable";
const STOP_STAGE_FORK_REFRESH: &str = "after post-inventory fork refresh";
const STOP_STAGE_FORK_ACKNOWLEDGEMENT: &str = "after frozen fork acknowledgement";
const STOP_STAGE_FIRST_AUTHORITY_CHECK: &str = "after initial kernel EOF denial";
const STOP_STAGE_PARTIAL_COVERAGE: &str = "after partial fresh coverage";
const STOP_STAGE_FRESH_COVERAGE: &str = "after fresh coverage settlement";

#[test]
fn incremental_exact_positive_authority_precedes_kernel_eof_fallback() {
    let mut fixture = ExactGateFixture::spawn(DUAL_AUTHORITY_CAPABILITY, false);
    let mut tree = fixture.take_tree();
    let expected = tree.captured_targets()[usize::default()].0;
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    tree.capability_scan_pending = true;
    tree.lineage_integrity_lost = false;
    tree.pty_scan.mark_released_needs_scan_for_test();

    let targets = tree.captured_targets();
    let authority = tree
        .captured_stopped_closure_authority(&targets)
        .unwrap()
        .unwrap();
    assert!(!tree.exact_stopped_closure_can_kill(&authority));
    assert!(tree.incremental_stopped_closure_can_kill(&authority));
    assert!(tree.kernel_eof_stopped_closure_can_kill(&authority));
    assert!(matches!(
        exact_stopped_candidate(&mut tree).unwrap(),
        Some(FreezeClosureCandidate::IncrementalExact(closure)) if closure == [expected]
    ));
}

#[test]
fn kernel_eof_remains_the_fallback_without_exact_positive_authority() {
    let mut fixture = ExactGateFixture::spawn(KERNEL_ONLY_CAPABILITY, false);
    let mut tree = fixture.take_tree();
    let expected = tree.captured_targets()[usize::default()].0;
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    tree.capability_scan_pending = true;
    tree.lineage_integrity_lost = false;
    tree.pty_scan = Default::default();
    tree.pty_scan.mark_released_needs_scan_for_test();

    let targets = tree.captured_targets();
    let authority = tree
        .captured_stopped_closure_authority(&targets)
        .unwrap()
        .unwrap();
    assert!(!tree.exact_stopped_closure_can_kill(&authority));
    assert!(!tree.incremental_stopped_closure_can_kill(&authority));
    assert!(tree.kernel_eof_stopped_closure_can_kill(&authority));
    assert!(matches!(
        exact_stopped_candidate(&mut tree).unwrap(),
        Some(FreezeClosureCandidate::KernelEof(closure)) if closure == [expected]
    ));
}

#[test]
fn retained_debt_denies_nonempty_kernel_eof_before_coverage_completes() {
    let mut fixture = ExactGateFixture::spawn(INCOMPLETE_COVERAGE_CAPABILITY, false);
    let mut tree = fixture.take_tree();
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    tree.capability_scan_pending = true;
    tree.lineage_integrity_lost = false;
    tree.pty_scan.mark_released_needs_scan_for_test();
    tree.frozen_fork_reconciliation_required = true;

    assert_only_fork_debt_denies_kernel_eof(&mut tree);
}

#[test]
fn production_refresh_retains_debt_for_incomplete_lineage_coverage() {
    let (mut fixture, mut tree, omitted) = debt_ready_fixture(LINEAGE_GAP_CAPABILITY);
    fixture.install_inventory_omitting(omitted);

    tree.refresh().unwrap();

    assert_production_partial_coverage(&mut tree, FrozenForkCoverage::MISSING_LINEAGE);
}

#[test]
fn production_refresh_retains_debt_for_incomplete_capability_coverage() {
    let (mut fixture, mut tree, _omitted) = debt_ready_fixture(CAPABILITY_GAP_CAPABILITY);
    fixture.install_observed_inventory();
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    tree.capability_scan_pending = true;
    tree.lineage_integrity_lost = false;
    tree.pty_scan.mark_released_needs_scan_for_test();

    tree.refresh_for_kernel_eof().unwrap();

    assert_production_partial_coverage(&mut tree, FrozenForkCoverage::MISSING_CAPABILITY);
}

#[test]
fn production_refresh_retains_debt_for_incomplete_publication_coverage() {
    let (mut fixture, mut tree, _omitted) = debt_ready_fixture(PUBLICATION_GAP_CAPABILITY);
    fixture.install_observed_inventory();
    tree.fail_next_event_reconcile();

    tree.refresh().unwrap();

    assert_production_partial_coverage(&mut tree, FrozenForkCoverage::MISSING_PUBLICATION);
}

#[test]
fn post_inventory_fork_debt_blocks_kernel_eof_until_fresh_coverage() {
    let mut fixture = ExactGateFixture::spawn(POST_INVENTORY_FORK_CAPABILITY, false);
    let mut tree = fixture.take_tree();
    tree.reconcile_retained_root_exit().unwrap();
    assert!(tree.root_exit_is_proven());
    tree.discovery_epoch = DiscoveryScanEpoch::Covered;
    tree.capability_scan_pending = false;
    tree.lineage_settled = true;
    tree.lineage_integrity_lost = false;
    let source = tree.captured_targets()[usize::default()].0;
    let _fork = tree.queue_fork_after_next_inventory(source);

    tree.refresh().unwrap();
    assert_fork_source_stopped(source, STOP_STAGE_FORK_REFRESH, &tree);
    assert!(tree.events.has_unresolved_fork());
    assert!(tree.events.requires_reconciliation());
    tree.discovery_epoch = DiscoveryScanEpoch::Covered;
    tree.capability_scan_pending = false;
    tree.lineage_settled = true;
    let acknowledged = tree.acknowledge_frozen_forks(&[source]);
    assert!(
        acknowledged,
        "{FROZEN_FORK_ACKNOWLEDGEMENT_DETAIL}: {:#?}",
        tree.state_snapshot_for_test()
    );
    assert!(!tree.events.has_unresolved_fork());
    assert!(tree.frozen_fork_reconciliation_required);
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert!(tree.capability_scan_pending);
    assert_fork_source_stopped(source, STOP_STAGE_FORK_ACKNOWLEDGEMENT, &tree);

    assert!(tree.events.requires_reconciliation());
    tree.pty_scan.mark_released_needs_scan_for_test();
    assert!(!kernel_eof_authorized(&mut tree));
    assert_fork_source_stopped(source, STOP_STAGE_FIRST_AUTHORITY_CHECK, &tree);
    assert_partial_coverage_is_insufficient(&mut tree);
    assert_fork_source_stopped(source, STOP_STAGE_PARTIAL_COVERAGE, &tree);
    settle_fresh_coverage(&mut tree);
    assert_fork_source_stopped(source, STOP_STAGE_FRESH_COVERAGE, &tree);
    tree.pty_scan.mark_released_needs_scan_for_test();
    assert!(kernel_eof_authorized(&mut tree));
}

include!("captured_tree_kernel_eof_debt_support.rs");
