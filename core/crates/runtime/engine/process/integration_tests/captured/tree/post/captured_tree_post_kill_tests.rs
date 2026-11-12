use super::*;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess};
use crate::process::{LiveProcessIdentity, install_test_match_failure};

const MATCH_FAILURE_DETAIL: &str = "injected post-kill generation query failure";
const ROOT_EXIT_RECONCILIATION_DETAIL: &str =
    "post-kill refresh did not reconcile a root without wait authority";
const LATE_TARGET_ACCEPTED_DETAIL: &str =
    "fresh discovery accepted a target outside the killed exact closure";
const PUBLICATION_FENCE_BYPASSED_DETAIL: &str =
    "fresh discovery refroze before publishing the target watcher";

#[test]
fn live_generation_wait_refreezes_at_the_bound() {
    let process = TestProcess::spawn();
    let live = live_identity(&process);
    let tree = tree_for(&process);
    let mut wait = exact_wait_for_tree(live, &tree);

    for _poll in 0..crate::process::constants::POST_KILL_EXACT_GENERATION_POLL_LIMIT {
        assert!(matches!(
            wait.observe_exact_generations(&tree).unwrap(),
            Some(PostKillObservation::Pending)
        ));
    }
    assert!(matches!(
        wait.observe_exact_generations(&tree).unwrap(),
        Some(PostKillObservation::Refreeze)
    ));
}

#[test]
fn dead_generation_yields_once_before_reconciliation() {
    let mut process = TestProcess::spawn();
    let live = live_identity(&process);
    let tree = tree_for(&process);
    let mut wait = exact_wait_for_tree(live, &tree);
    process.terminate();

    assert!(matches!(
        wait.observe_exact_generations(&tree).unwrap(),
        Some(PostKillObservation::Pending)
    ));
    assert!(wait.observe_exact_generations(&tree).unwrap().is_none());
}

#[test]
fn post_kill_refresh_reconciles_root_without_wait_authority() {
    let mut process = TestProcess::spawn();
    let live = live_identity(&process);
    let mut tree = tree_for(&process);
    let mut wait = exact_wait_for_tree(live, &tree);
    process.terminate();

    let _observation = wait.observe(&mut tree).unwrap();

    assert!(
        tree.root_exit_is_proven(),
        "{ROOT_EXIT_RECONCILIATION_DETAIL}"
    );
}

#[test]
fn generation_query_failure_preserves_exact_phase() {
    let process = TestProcess::spawn();
    let live = live_identity(&process);
    let tree = tree_for(&process);
    let mut wait = exact_wait_for_tree(live, &tree);
    {
        let _failure = install_test_match_failure(MATCH_FAILURE_DETAIL);
        let error = wait.observe_exact_generations(&tree).unwrap_err();
        assert!(error.to_string().contains(MATCH_FAILURE_DETAIL));
    }
    assert!(matches!(
        wait.observe_exact_generations(&tree).unwrap(),
        Some(PostKillObservation::Pending)
    ));
}

#[test]
fn fresh_discovery_publishes_the_target_watcher_before_refreeze() {
    let process = TestProcess::spawn();
    let late = TestProcess::spawn();
    let live = live_identity(&process);
    let mut tree = tree_for(&process);
    tree.bind_repeating_process_inventory_for_test(&[
        (process.identity, NO_PARENT_PID, process.identity.pid),
        (late.identity, process.identity.pid, process.identity.pid),
    ]);
    let mut wait = exact_wait_for_tree(live, &tree);

    assert!(
        matches!(
            wait.observe(&mut tree).unwrap(),
            PostKillObservation::Pending
        ),
        "{PUBLICATION_FENCE_BYPASSED_DETAIL}"
    );
    assert!(!tree.events.inventory_publication_is_current());
    assert!(
        matches!(
            wait.observe(&mut tree).unwrap(),
            PostKillObservation::Refreeze
        ),
        "{LATE_TARGET_ACCEPTED_DETAIL}"
    );
}

fn live_identity(process: &TestProcess) -> LiveProcessIdentity {
    LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap()
}

fn tree_for(process: &TestProcess) -> CapturedProcessTree {
    let mut tree = CapturedProcessTree::capture(process.identity).unwrap();
    tree.authorize_destructive_for_test();
    tree
}

fn exact_wait_for_tree(
    identity: LiveProcessIdentity,
    tree: &CapturedProcessTree,
) -> PostKillExactClosure {
    let kill = super::super::super::termination::ExactClosureKill::for_test(
        vec![(identity, tree.root())],
        tree.closure_version.clone(),
        super::super::super::termination::ExactKillAuthority::ExactPositive,
    );
    PostKillExactClosure::exact(kill)
}
