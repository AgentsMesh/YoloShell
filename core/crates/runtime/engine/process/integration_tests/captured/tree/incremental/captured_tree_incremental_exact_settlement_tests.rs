use super::exact_gate_fixture::ExactGateFixture;
use super::{FreezeClosureCandidate, exact_stopped_candidate};
use crate::process::captured_tree_macos_wait::{
    POLL_INTERVAL, TEST_TIMEOUT, wait_for_exit_unreaped,
};
use crate::process::inventory_test_support::TestProcess;
use crate::process::{CapturedProcessTree, FreezePoll, KillPoll, LiveProcessIdentity};
use std::time::Instant;

const TEST_CAPABILITY: &str = "incremental-exact-settlement";
const EXPECTED_TARGET_DETAIL: &str = "fixture did not expose one exact holder";
const EXPECTED_INCREMENTAL_DETAIL: &str = "freeze did not select incremental exact basis";
const SETTLEMENT_TIMEOUT_DETAIL: &str = "incremental exact closure did not settle";
const RELEASED_NEEDS_SCAN_STATE: &str = "ReleasedNeedsScan";
const PRE_KILL_DEBT_DETAIL: &str = "incremental freeze consumed global reconciliation debt";
const FRESH_EPOCH_DETAIL: &str = "post-kill exact lane reused pre-kill discovery coverage";

#[test]
fn released_negative_scan_does_not_block_exact_tracked_settlement() {
    let mut fixture = ExactGateFixture::spawn(TEST_CAPABILITY, false);
    let mut tree = fixture.take_tree();
    let targets = tree.captured_targets();
    assert_eq!(targets.len(), 1, "{EXPECTED_TARGET_DETAIL}");
    let holder = targets[0].0;
    let holder_durable = targets[0].1;
    install_gone_fork(&mut tree, holder);
    tree.events.invalidate();

    assert!(matches!(
        exact_stopped_candidate(&mut tree).unwrap(),
        Some(FreezeClosureCandidate::IncrementalExact(closure)) if closure == [holder]
    ));
    let frozen = freeze_incremental(tree);
    assert!(
        !frozen.state_snapshot_for_test().discovery_coverage_complete,
        "{PRE_KILL_DEBT_DETAIL}"
    );
    let version = frozen.closure_version();
    let mut pending = match frozen.kill_after_output_fence(version).unwrap() {
        super::super::KillCommitPoll::Pending(pending) => pending,
        super::super::KillCommitPoll::Refreeze(_) => panic!("{EXPECTED_INCREMENTAL_DETAIL}"),
    };
    assert!(
        !pending
            .state_snapshot_for_test()
            .discovery_coverage_complete,
        "{FRESH_EPOCH_DETAIL}"
    );
    wait_for_exit_unreaped(holder_durable);
    fixture.install_observed_inventory();

    let deadline = Instant::now() + TEST_TIMEOUT;
    let mut final_pending = None;
    loop {
        pending = match pending.poll().unwrap() {
            KillPoll::Pending(next) => {
                final_pending = Some(next.state_snapshot_for_test());
                assert!(Instant::now() < deadline, "{SETTLEMENT_TIMEOUT_DETAIL}");
                std::thread::sleep(POLL_INTERVAL);
                next
            }
            KillPoll::Gone(_) => break,
            KillPoll::Refreeze(_) => panic!("{SETTLEMENT_TIMEOUT_DETAIL}"),
        };
    }
    let snapshot = final_pending.expect(SETTLEMENT_TIMEOUT_DETAIL);
    assert!(snapshot.capability_scan_pending);
    assert_eq!(
        snapshot.capability_liveness.as_deref(),
        Some(RELEASED_NEEDS_SCAN_STATE)
    );
    assert!(snapshot.discovery_coverage_complete);
    assert!(snapshot.captured.is_empty());
    assert!(!snapshot.pty_master_released);
}

fn install_gone_fork(tree: &mut CapturedProcessTree, holder: LiveProcessIdentity) {
    let mut source = TestProcess::spawn();
    let source_live = LiveProcessIdentity::capture(source.identity)
        .unwrap()
        .unwrap();
    let _ = tree.events.reconcile_inventory([holder, source_live]);
    tree.events.inject_unresolved_fork(source_live);
    source.terminate();
    let _ = tree.events.reconcile_inventory([holder]);
    let _ = tree.events.reconcile_inventory([holder]);
}

fn freeze_incremental(tree: CapturedProcessTree) -> super::super::FrozenProcessTree {
    let freezing = match tree.begin_force().poll().unwrap() {
        FreezePoll::Pending(freezing) => freezing,
        FreezePoll::Frozen(_) => panic!("{EXPECTED_INCREMENTAL_DETAIL}"),
    };
    match freezing.poll().unwrap() {
        FreezePoll::Frozen(frozen) => frozen,
        FreezePoll::Pending(_) => panic!("{EXPECTED_INCREMENTAL_DETAIL}"),
    }
}
