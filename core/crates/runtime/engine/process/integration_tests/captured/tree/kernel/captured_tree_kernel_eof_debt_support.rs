use super::exact_gate_authority_tests::settle_exact_coverage;

fn settle_fresh_coverage(tree: &mut CapturedProcessTree) {
    let before = tree.state_snapshot_for_test();
    assert!(
        before.frozen_fork_reconciliation_required && !before.discovery_coverage_complete,
        "{FRESH_COVERAGE_PRECONDITION_DETAIL}: {before:#?}"
    );

    settle_exact_coverage(tree);

    let after = tree.state_snapshot_for_test();
    assert!(
        after.discovery_coverage_complete
            && after.capability_coverage_complete
            && after.publication_quiescent
            && !after.event_reconciliation_pending
            && after.lineage_settled
            && !after.lineage_integrity_lost
            && after.unresolved_fork_lineages.is_empty()
            && !after.frozen_fork_reconciliation_required,
        "{FRESH_COVERAGE_INVARIANT_DETAIL}: {after:#?}"
    );
}

fn assert_partial_coverage_is_insufficient(tree: &mut CapturedProcessTree) {
    tree.force_next_frozen_fork_capability_gap_for_test();
    tree.refresh().unwrap();
    assert_fork_debt_retained(tree);
    tree.force_next_frozen_fork_publication_gap_for_test();
    tree.refresh().unwrap();
    assert_fork_debt_retained(tree);
    tree.force_next_frozen_fork_lineage_gap_for_test();
    tree.refresh().unwrap();
    assert_fork_debt_retained(tree);
}

fn debt_ready_fixture(
    capability: &str,
) -> (ExactGateFixture, CapturedProcessTree, ProcessIdentity) {
    let mut fixture = ExactGateFixture::spawn(capability, false);
    let mut tree = fixture.take_tree();
    let durable = tree.captured_targets()[usize::default()].1;
    let baseline = tree.state_snapshot_for_test();
    assert!(
        baseline.lineage_settled
            && baseline.capability_coverage_complete
            && baseline.publication_quiescent
            && baseline.event_gate_present,
        "partial-coverage fixture was not fully settled: {baseline:#?}"
    );
    tree.frozen_fork_reconciliation_required = true;
    (fixture, tree, durable)
}

fn assert_production_partial_coverage(
    tree: &mut CapturedProcessTree,
    expected: FrozenForkCoverage,
) {
    let snapshot = tree.state_snapshot_for_test();
    let observed = FrozenForkCoverage {
        lineage_complete: snapshot.lineage_settled,
        capability_coverage_complete: snapshot.capability_coverage_complete,
        publication_quiescent: snapshot.publication_quiescent,
    };
    assert_eq!(observed, expected, "{snapshot:#?}");
    assert_fork_debt_retained(tree);
    assert!(!kernel_eof_authorized(tree), "{snapshot:#?}");
}

fn assert_fork_debt_retained(tree: &CapturedProcessTree) {
    let snapshot = tree.state_snapshot_for_test();
    assert!(
        snapshot.frozen_fork_reconciliation_required,
        "{PARTIAL_COVERAGE_DEBT_DETAIL}: {snapshot:#?}"
    );
}

fn assert_fork_source_stopped(
    identity: LiveProcessIdentity,
    stage: &str,
    tree: &CapturedProcessTree,
) {
    let state = identity.execution_state().unwrap();
    assert_eq!(
        state,
        crate::process::live_identity::ProcessExecutionState::Stopped,
        "{STOPPED_CLOSURE_AUTHORITY_DETAIL} {stage}: {:#?}",
        tree.state_snapshot_for_test()
    );
}

fn assert_only_fork_debt_denies_kernel_eof(tree: &mut CapturedProcessTree) {
    assert!(tree.frozen_fork_reconciliation_required);
    tree.frozen_fork_reconciliation_required = false;
    assert!(kernel_eof_authorized(tree));

    tree.frozen_fork_reconciliation_required = true;
    assert!(!kernel_eof_authorized(tree));
}

fn kernel_eof_authorized(tree: &mut CapturedProcessTree) -> bool {
    let targets = tree.captured_targets();
    let snapshot = tree.state_snapshot_for_test();
    let authority = tree
        .captured_stopped_closure_authority(&targets)
        .unwrap()
        .unwrap_or_else(|| panic!("{STOPPED_CLOSURE_AUTHORITY_DETAIL}: {snapshot:#?}"));
    tree.kernel_eof_stopped_closure_can_kill(&authority)
}
