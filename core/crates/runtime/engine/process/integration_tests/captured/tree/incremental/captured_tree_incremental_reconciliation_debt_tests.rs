use super::exact_gate_fixture::ExactGateFixture;
use super::{FreezeClosureCandidate, exact_stopped_candidate};
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::inventory_test_support::TestProcess;
use crate::process::{
    CapturedProcessTree, FreezePoll, FrozenProcessTree, KillCommitPoll, KillPoll,
    LiveProcessIdentity,
};
use std::time::Instant;

const TEST_CAPABILITY: &str = "incremental-independent-reconciliation-debt";
const EXACT_HOLDER_REQUIRED_DETAIL: &str = "fixture did not retain one exact holder";
const INCREMENTAL_REQUIRED_DETAIL: &str = "independent debt did not select incremental authority";
const PENDING_REQUIRED_DETAIL: &str = "incremental closure skipped freeze confirmation";
const FROZEN_REQUIRED_DETAIL: &str = "incremental closure did not freeze";
const KILL_PENDING_REQUIRED_DETAIL: &str = "incremental kill did not require reconciliation";
const DEBT_LOST_DETAIL: &str = "incremental authority consumed reconciliation debt";
const MASTER_RELEASED_DETAIL: &str = "PTY master was released before global reconciliation";
const LATE_PROCESS_MISSING_DETAIL: &str = "global reconciliation missed the hidden late process";
const POST_KILL_TIMEOUT_DETAIL: &str = "post-kill reconciliation did not refreeze the late process";
const REFRESH_FAILURE_DETAIL: &str = "injected freeze coverage refresh failure";

#[test]
fn incremental_kill_preserves_independent_debt_until_late_process_refreeze() {
    let mut fixture = ExactGateFixture::spawn(TEST_CAPABILITY, true);
    let mut tree = fixture.take_tree();
    let late = fixture.late_identity().unwrap();
    let targets = tree.captured_targets();
    assert_eq!(targets.len(), 1, "{EXACT_HOLDER_REQUIRED_DETAIL}");
    let (holder_live, holder) = targets[usize::default()];
    let isolated_source = install_isolated_gone_fork(&mut tree, holder_live);
    tree.events.invalidate();

    assert_incremental_authority(&mut tree, holder_live, isolated_source);
    let frozen = freeze_incremental(tree);
    assert_debt_retained(&frozen.tree, isolated_source);
    let version = frozen.closure_version();
    let pending = match frozen.kill_after_output_fence(version).unwrap() {
        KillCommitPoll::Pending(pending) => pending,
        KillCommitPoll::Refreeze(_) => panic!("{KILL_PENDING_REQUIRED_DETAIL}"),
    };
    crate::process::captured_tree_macos_wait::wait_for_exit_unreaped(holder);

    assert!(late.matches(), "{LATE_PROCESS_MISSING_DETAIL}");
    assert_debt_retained(&pending.tree, isolated_source);
    fixture.install_observed_inventory();
    let freezing = wait_for_late_refreeze(pending, late);
    let snapshot = freezing.state_snapshot_for_test();
    assert!(!snapshot.pty_master_released, "{MASTER_RELEASED_DETAIL}");
    assert!(
        snapshot.captured.contains(&late),
        "{LATE_PROCESS_MISSING_DETAIL}"
    );
}

#[test]
fn failed_freeze_coverage_refresh_returns_independent_debt() {
    let mut fixture = ExactGateFixture::spawn(TEST_CAPABILITY, true);
    let mut tree = fixture.take_tree();
    let targets = tree.captured_targets();
    assert_eq!(targets.len(), 1, "{EXACT_HOLDER_REQUIRED_DETAIL}");
    let holder = targets[usize::default()].0;
    let isolated_source = install_isolated_gone_fork(&mut tree, holder);
    tree.events.invalidate();
    tree.fail_next_refresh(REFRESH_FAILURE_DETAIL);

    let error = super::refresh_freeze_coverage(&mut tree).unwrap_err();

    assert_eq!(error.to_string(), REFRESH_FAILURE_DETAIL);
    assert_debt_retained(&tree, isolated_source);
}

#[test]
fn incremental_freeze_precedes_global_coverage_refresh() {
    let mut fixture = ExactGateFixture::spawn(TEST_CAPABILITY, false);
    let mut tree = fixture.take_tree();
    let targets = tree.captured_targets();
    assert_eq!(targets.len(), 1, "{EXACT_HOLDER_REQUIRED_DETAIL}");
    let holder = targets[usize::default()].0;
    let isolated_source = install_isolated_gone_fork(&mut tree, holder);
    tree.events.invalidate();
    assert_incremental_authority(&mut tree, holder, isolated_source);
    tree.fail_next_refresh(REFRESH_FAILURE_DETAIL);

    let mut frozen = freeze_incremental(tree);

    assert_debt_retained(&frozen.tree, isolated_source);
    assert!(
        !frozen.state_snapshot_for_test().discovery_coverage_complete,
        "{DEBT_LOST_DETAIL}"
    );
    let error = frozen.tree.refresh().unwrap_err();
    assert_eq!(error.to_string(), REFRESH_FAILURE_DETAIL);
}

pub(super) fn install_isolated_gone_fork(
    tree: &mut CapturedProcessTree,
    holder: LiveProcessIdentity,
) -> LiveProcessIdentity {
    let mut source = TestProcess::spawn();
    let source_live = LiveProcessIdentity::capture(source.identity)
        .unwrap()
        .unwrap();
    let _ = tree.events.reconcile_inventory([holder, source_live]);
    assert!(tree.events.requires_reconciliation());
    tree.events.inject_unresolved_fork(source_live);
    source.terminate();
    let _ = tree.events.reconcile_inventory([holder]);
    assert!(tree.events.requires_reconciliation());
    assert!(tree.events.reconcile_inventory([holder]));
    assert!(!tree.events.requires_reconciliation());
    assert!(tree.events.exact_closure_isolated(&[holder]));
    source_live
}

fn assert_incremental_authority(
    tree: &mut CapturedProcessTree,
    holder: LiveProcessIdentity,
    isolated_source: LiveProcessIdentity,
) {
    let candidate = exact_stopped_candidate(tree).unwrap();
    assert!(
        matches!(candidate, Some(FreezeClosureCandidate::IncrementalExact(closure)) if closure == [holder]),
        "{INCREMENTAL_REQUIRED_DETAIL}: {:#?}",
        tree.state_snapshot_for_test()
    );
    assert_debt_retained(tree, isolated_source);
}

fn freeze_incremental(tree: CapturedProcessTree) -> FrozenProcessTree {
    let freezing = match tree.begin_force().poll().unwrap() {
        FreezePoll::Pending(freezing) => freezing,
        FreezePoll::Frozen(_) => panic!("{PENDING_REQUIRED_DETAIL}"),
    };
    match freezing.poll().unwrap() {
        FreezePoll::Frozen(frozen) => frozen,
        FreezePoll::Pending(_) => panic!("{FROZEN_REQUIRED_DETAIL}"),
    }
}

fn assert_debt_retained(tree: &CapturedProcessTree, source: LiveProcessIdentity) {
    let snapshot = tree.state_snapshot_for_test();
    assert!(snapshot.event_reconciliation_pending, "{DEBT_LOST_DETAIL}");
    assert!(
        snapshot
            .unresolved_fork_lineages
            .contains(&source.lineage_id()),
        "{DEBT_LOST_DETAIL}"
    );
    assert!(!snapshot.pty_master_released, "{MASTER_RELEASED_DETAIL}");
}

fn wait_for_late_refreeze(
    mut pending: crate::process::KillPendingProcessTree,
    late: crate::process::ProcessIdentity,
) -> crate::process::FreezingProcessTree {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        pending = match pending.poll().unwrap() {
            KillPoll::Refreeze(freezing) => {
                assert!(
                    freezing.tree.identities().contains(&late),
                    "{LATE_PROCESS_MISSING_DETAIL}"
                );
                return freezing;
            }
            KillPoll::Pending(pending) => pending,
            KillPoll::Gone(_) => panic!("{MASTER_RELEASED_DETAIL}"),
        };
        assert!(
            !pending.tree.pty_master_released(),
            "{MASTER_RELEASED_DETAIL}"
        );
        assert!(Instant::now() < deadline, "{POST_KILL_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}
