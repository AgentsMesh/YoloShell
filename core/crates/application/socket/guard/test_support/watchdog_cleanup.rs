use super::constants::*;
use anyhow::{Result, ensure};
use std::time::Instant;
use yolo_process::CapturedProcessTree;

#[path = "watchdog_cleanup_epoch.rs"]
mod epoch;

pub(super) fn settle(tree: &mut CapturedProcessTree) -> Result<()> {
    tree.begin_discovery_cleanup();
    let mut refresh_failure = None;
    let mut signal_failure = None;
    epoch::prepare_for_signals(tree, &mut refresh_failure);
    signal_live(tree, libc::SIGTERM, &mut signal_failure);
    let graceful_deadline = Instant::now() + GRACEFUL_CLEANUP_WINDOW;
    let graceful_settled = drive(
        tree,
        libc::SIGTERM,
        graceful_deadline,
        &mut refresh_failure,
        &mut signal_failure,
    );
    let settled = if graceful_settled {
        true
    } else {
        drive(
            tree,
            libc::SIGKILL,
            Instant::now() + WATCHDOG_FORCE_SETTLEMENT_TIMEOUT,
            &mut refresh_failure,
            &mut signal_failure,
        )
    };
    ensure!(settled, WATCHDOG_TREE_DETAIL);
    ensure!(!has_live(tree)?, WATCHDOG_TREE_DETAIL);
    ensure!(tree.discovery_is_settled(), WATCHDOG_TREE_DETAIL);
    let failures = [refresh_failure, signal_failure]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    ensure!(failures.is_empty(), failures.join(FAILURE_SEPARATOR));
    Ok(())
}

fn drive(
    tree: &mut CapturedProcessTree,
    signal: i32,
    deadline: Instant,
    refresh_failure: &mut Option<String>,
    signal_failure: &mut Option<String>,
) -> bool {
    tree.begin_discovery_cleanup();
    while Instant::now() < deadline {
        epoch::refresh(tree, refresh_failure);
        match tree.discovery_publication_ready() {
            Ok(true) => return true,
            Ok(false) => {}
            Err(error) => {
                record_first(refresh_failure, Err(error));
                signal_captured(tree, signal, signal_failure);
                std::thread::sleep(PROCESS_POLL_INTERVAL);
                continue;
            }
        }
        match has_live(tree) {
            Ok(true) => {
                signal_captured(tree, signal, signal_failure);
            }
            Ok(false) if tree.discovery_is_settled() => tree.begin_discovery_cleanup(),
            Ok(false) => {}
            Err(error) => {
                record_first(refresh_failure, Err(error));
                signal_captured(tree, signal, signal_failure);
            }
        }
        std::thread::sleep(PROCESS_POLL_INTERVAL);
    }
    false
}

fn signal_live(tree: &mut CapturedProcessTree, signal: i32, failure: &mut Option<String>) {
    if !live_or_pending(tree, failure) {
        return;
    }
    signal_captured(tree, signal, failure);
}

fn signal_captured(tree: &mut CapturedProcessTree, signal: i32, failure: &mut Option<String>) {
    #[cfg(feature = "adversarial-test-hooks")]
    if signal == libc::SIGKILL {
        record_first(failure, super::fault::before_force_signal());
    }
    record_first(failure, tree.terminate_captured(signal).map(|_| ()));
    tree.begin_discovery_cleanup();
}

pub(super) fn live_or_pending(tree: &CapturedProcessTree, failure: &mut Option<String>) -> bool {
    match has_live(tree) {
        Ok(live) => live,
        Err(error) => {
            record_first(failure, Err(error));
            true
        }
    }
}

pub(super) fn has_live(tree: &CapturedProcessTree) -> Result<bool> {
    tree.live_identities()
        .map(|identities| !identities.is_empty())
}

pub(super) fn record_first(failure: &mut Option<String>, result: Result<()>) {
    if failure.is_none()
        && let Err(error) = result
    {
        *failure = Some(error.to_string());
    }
}
