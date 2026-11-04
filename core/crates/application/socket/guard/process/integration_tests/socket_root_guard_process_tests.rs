#[path = "../test_support/constants.rs"]
mod constants;
use yolo_socket_guard_fault_contract as fault_contract;
#[path = "fixture.rs"]
mod fixture;
#[path = "../test_support/guard_process.rs"]
mod guard_process;
#[path = "../test_support/guard_process_drop.rs"]
mod guard_process_drop;
#[path = "../test_support/process_identity_record.rs"]
mod process_identity_record;
#[path = "../test_support/process_support.rs"]
mod process_support;

use guard_process::{FailureAction, GuardProcess, LeafState};
use std::time::Instant;

#[test]
fn normal_completion_reaps_watchdog_and_removes_socket_root() {
    let mut guard = GuardProcess::spawn(LeafState::None, FailureAction::None);
    guard.wait_for_success();
}

#[test]
fn exact_guard_kill_cannot_orphan_a_double_fork_leaf() {
    let mut guard = GuardProcess::spawn(LeafState::Running, FailureAction::GuardKill);
    guard.trigger_failure();
    guard.wait_for_failure_cleanup();
}

#[test]
fn runner_exit_without_destructors_still_cleans_a_double_fork_leaf() {
    let mut guard = GuardProcess::spawn(LeafState::Running, FailureAction::Exit);
    guard.trigger_failure();
    guard.wait_for_failure_cleanup();
}

#[test]
fn runner_panic_drop_still_cleans_a_double_fork_leaf() {
    let mut guard = GuardProcess::spawn(LeafState::Running, FailureAction::Panic);
    guard.trigger_failure();
    guard.wait_for_failure_cleanup();
}

#[test]
fn stopped_setsid_leaf_is_force_settled_after_group_timeout() {
    let mut guard = GuardProcess::spawn(LeafState::Stopped, FailureAction::GuardKill);
    guard.trigger_failure();
    guard.wait_for_failure_cleanup();
}

#[test]
fn early_watchdog_death_falls_back_to_local_tree_and_root_custody() {
    let mut guard = GuardProcess::spawn(LeafState::Running, FailureAction::WatchdogKill);
    guard.trigger_failure();
    guard.wait_for_failure_cleanup();
}

#[test]
fn setup_abort_recovers_unpublished_process_and_root_evidence() {
    let guard = GuardProcess::spawn_unready(LeafState::Running);
    let guard_identity = guard.guard_identity;
    let (watchdog, leaf, root) = guard.wait_for_setup_evidence();
    assert!(guard.watchdog.is_none() && guard.leaf.is_none() && guard.socket_root.is_none());
    drop(guard);
    let deadline = Instant::now() + constants::CLEANUP_TIMEOUT;
    while guard_identity.matches() || watchdog.matches() || leaf.matches() || root.exists() {
        assert!(
            Instant::now() < deadline,
            "{}",
            constants::CLEANUP_TIMEOUT_DETAIL
        );
        std::thread::sleep(constants::POLL_INTERVAL);
    }
}

#[test]
fn term_handler_handoff_is_found_by_the_post_signal_epoch() {
    let mut guard = GuardProcess::spawn(LeafState::ForkOnTerm, FailureAction::GuardKill);
    guard.trigger_failure();
    guard.wait_for_failure_cleanup();
    assert!(guard.late_leaf.is_some());
}

#[test]
fn pre_kill_handoff_is_found_by_the_restarted_discovery_epoch() {
    let mut guard = GuardProcess::spawn(LeafState::ForkBeforeForce, FailureAction::GuardKill);
    guard.trigger_failure();
    guard.wait_for_failure_cleanup();
    assert!(guard.late_leaf.is_some());
}

#[test]
fn post_refresh_handoff_is_found_from_the_historical_live_witness() {
    let mut guard = GuardProcess::spawn(LeafState::ForkAfterRefresh, FailureAction::GuardKill);
    guard.trigger_failure();
    guard.wait_for_failure_cleanup();
    let milestones = guard.refresh_milestones().unwrap();
    assert!(milestones.complete(), "{milestones:?}");
    assert!(guard.refresh_holder.is_some());
    assert!(guard.late_leaf.is_some());
    assert!(guard.refresh_postcheck_witnessed());
}
