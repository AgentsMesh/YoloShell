use super::super::constants::*;
use super::record_first;
use std::time::Instant;
use yolo_process::CapturedProcessTree;

#[cfg(feature = "adversarial-test-hooks")]
use yolo_process::ProcessIdentity;

#[cfg(feature = "adversarial-test-hooks")]
const REFRESH_HANDOFF_POSTCHECK_DETAIL: &str =
    "refresh-handoff producer remained live after acknowledgement";

pub(super) fn prepare_for_signals(tree: &mut CapturedProcessTree, failure: &mut Option<String>) {
    let deadline = Instant::now() + WATCHDOG_DISCOVERY_PREPARATION_TIMEOUT;
    while !tree.discovery_coverage_is_complete() && Instant::now() < deadline {
        let _ = refresh_identities(tree, failure);
        if !tree.discovery_coverage_is_complete() {
            std::thread::sleep(PROCESS_POLL_INTERVAL);
        }
    }
    if !tree.discovery_coverage_is_complete() {
        record_first(failure, Err(anyhow::anyhow!(WATCHDOG_TREE_DETAIL)));
        return;
    }
    #[cfg(feature = "adversarial-test-hooks")]
    if discovery_arm(failure) {
        tree.begin_discovery_cleanup();
    }
}

pub(super) fn refresh(tree: &mut CapturedProcessTree, failure: &mut Option<String>) {
    #[cfg(not(feature = "adversarial-test-hooks"))]
    {
        let _ = refresh_identities(tree, failure);
    }

    #[cfg(feature = "adversarial-test-hooks")]
    {
        let before = live_identities(tree, failure);
        let observed = refresh_identities(tree, failure);
        let handoff =
            has_new_identity(before.as_deref(), observed.as_deref()) && refresh_handoff(failure);
        let after = live_identities(tree, failure);
        finish_handoff(failure, handoff, after);
    }
}

#[cfg(feature = "adversarial-test-hooks")]
fn finish_handoff(
    failure: &mut Option<String>,
    handoff: bool,
    after: Option<Vec<ProcessIdentity>>,
) {
    let live_after = has_live(after.as_deref());
    if handoff && live_after {
        record_first(
            failure,
            Err(anyhow::anyhow!(REFRESH_HANDOFF_POSTCHECK_DETAIL)),
        );
    }
    if handoff && !live_after {
        record_first(failure, super::super::fault::record_refresh_postcheck());
    }
}

#[cfg(feature = "adversarial-test-hooks")]
fn live_identities(
    tree: &CapturedProcessTree,
    failure: &mut Option<String>,
) -> Option<Vec<ProcessIdentity>> {
    match tree.live_identities() {
        Ok(identities) => Some(identities),
        Err(error) => {
            record_first(failure, Err(error));
            None
        }
    }
}

fn refresh_identities(
    tree: &mut CapturedProcessTree,
    failure: &mut Option<String>,
) -> Option<Vec<ProcessIdentity>> {
    match tree.refresh() {
        Ok(identities) => Some(identities),
        Err(error) => {
            record_first(failure, Err(error));
            None
        }
    }
}

#[cfg(feature = "adversarial-test-hooks")]
fn refresh_handoff(failure: &mut Option<String>) -> bool {
    match super::super::fault::after_new_live_refresh() {
        Ok(activated) => activated,
        Err(error) => {
            record_first(failure, Err(error));
            false
        }
    }
}

#[cfg(feature = "adversarial-test-hooks")]
fn discovery_arm(failure: &mut Option<String>) -> bool {
    match super::super::fault::after_empty_settled_refresh() {
        Ok(activated) => activated,
        Err(error) => {
            record_first(failure, Err(error));
            false
        }
    }
}

#[cfg(feature = "adversarial-test-hooks")]
fn has_new_identity(
    before: Option<&[ProcessIdentity]>,
    observed: Option<&[ProcessIdentity]>,
) -> bool {
    matches!((before, observed), (Some(before), Some(observed)) if observed.iter().any(|identity| !before.contains(identity)))
}

#[cfg(feature = "adversarial-test-hooks")]
fn has_live(identities: Option<&[ProcessIdentity]>) -> bool {
    identities.is_none_or(|identities| !identities.is_empty())
}

#[cfg(feature = "adversarial-test-hooks")]
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/watchdog_cleanup_epoch_tests.rs"]
mod tests;
