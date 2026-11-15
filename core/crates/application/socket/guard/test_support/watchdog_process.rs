use super::constants::*;
use super::direct_child::force_unreaped_direct_child;
use std::process::{Child, ChildStdin};
use std::time::{Duration, Instant};
use yolo_process::ProcessIdentity;

pub(super) fn settle_failed(
    child: Child,
    input: Option<ChildStdin>,
    identity: Option<ProcessIdentity>,
) {
    settle_failed_until(
        child,
        input,
        identity,
        Instant::now() + WATCHDOG_DROP_TIMEOUT,
        WATCHDOG_FORCE_REAP_TIMEOUT,
    );
}

fn settle_failed_until(
    mut child: Child,
    input: Option<ChildStdin>,
    identity: Option<ProcessIdentity>,
    drop_deadline: Instant,
    force_timeout: Duration,
) {
    drop(input.or_else(|| child.stdin.take()));
    if reap(&mut child, drop_deadline) {
        return;
    }
    let _ = force_unreaped_direct_child(&mut child, identity, WATCHDOG_REAP_DETAIL);
    let _ = reap(&mut child, Instant::now() + force_timeout);
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))] #[path = "../integration_tests/watchdog_process_tests.rs"]
mod tests;

fn reap(child: &mut Child, deadline: Instant) -> bool {
    loop {
        if child.try_wait().ok().flatten().is_some() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(PROCESS_POLL_INTERVAL);
    }
}
