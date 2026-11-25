use super::{
    ResilienceFreezingProcessTree, ResilienceFrozenProcessTree, ResilienceKillPendingProcessTree,
};
use crate::process::{LiveProcessIdentity, ProcessIdentity};
use std::collections::HashMap;
use std::time::Duration;

#[path = "resilience_force_cleanup_drop.rs"]
mod drop;
#[path = "resilience_force_cleanup_fallback.rs"]
mod fallback;
#[path = "resilience_force_cleanup_run.rs"]
mod run;
#[path = "resilience_force_cleanup_terminal.rs"]
mod terminal;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/resilience/resilience_force_cleanup_terminal_tests.rs"]
mod terminal_tests;

pub(super) const CLEANUP_CONFIRM_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const CLEANUP_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum CleanupIntent {
    Resume,
    Kill,
    Disarmed,
}

pub(super) struct RetainedProcessCleanup {
    targets: HashMap<LiveProcessIdentity, ProcessIdentity>,
    intent: CleanupIntent,
}

impl RetainedProcessCleanup {
    pub(super) fn new(targets: Vec<(LiveProcessIdentity, ProcessIdentity)>) -> Self {
        let mut cleanup = Self {
            targets: HashMap::new(),
            intent: CleanupIntent::Resume,
        };
        cleanup.absorb(targets);
        cleanup
    }

    pub(super) fn absorb(&mut self, targets: Vec<(LiveProcessIdentity, ProcessIdentity)>) {
        self.targets.extend(targets);
    }

    pub(super) fn commit_to_kill(&mut self) {
        self.intent = CleanupIntent::Kill;
    }

    pub(super) fn is_committed_to_kill(&self) -> bool {
        self.intent == CleanupIntent::Kill
    }

    pub(super) fn disarm(&mut self) {
        self.intent = CleanupIntent::Disarmed;
        self.targets.clear();
    }

    pub(super) fn sorted_targets(&self) -> Vec<(LiveProcessIdentity, ProcessIdentity)> {
        let mut targets = self
            .targets
            .iter()
            .map(|(live, durable)| (live.clone(), *durable))
            .collect::<Vec<_>>();
        targets.sort_by_key(|(live, durable)| (durable.pid, durable.start_time, live.lineage_id()));
        targets
    }
}
