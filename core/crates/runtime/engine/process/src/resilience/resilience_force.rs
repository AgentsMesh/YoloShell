use super::ResilienceProcessTree;
use crate::process::{
    FreezingProcessTree, FrozenProcessTree, KillPendingProcessTree, ProcessIdentity,
    TrackedProducersGone,
};

#[path = "resilience_force_cleanup.rs"]
mod cleanup;
#[path = "resilience_force_error.rs"]
mod error;
#[path = "resilience_force_kill_transition.rs"]
mod kill_transition;
#[cfg(all(test, feature = "yolo-integration-tests"))]
pub(crate) use kill_transition::install_test_kill_commit_handoff_panic;
#[path = "resilience_force_transition.rs"]
mod transition;
use cleanup::RetainedProcessCleanup;
pub use error::ResilienceTransitionError;

pub struct ResilienceFreezingProcessTree {
    inner: Option<FreezingProcessTree>,
    cleanup: Option<RetainedProcessCleanup>,
}

pub struct ResilienceFrozenProcessTree {
    inner: Option<FrozenProcessTree>,
    cleanup: Option<RetainedProcessCleanup>,
}

pub struct ResilienceKillPendingProcessTree {
    inner: Option<KillPendingProcessTree>,
    cleanup: Option<RetainedProcessCleanup>,
}

pub struct ResilienceProducersGone {
    inner: TrackedProducersGone,
}

pub enum ResilienceFreezePoll {
    Pending(ResilienceFreezingProcessTree),
    Frozen(ResilienceFrozenProcessTree),
}

pub enum ResilienceKillCommitPoll {
    Refreeze(ResilienceFreezingProcessTree),
    Pending(ResilienceKillPendingProcessTree),
}

pub enum ResilienceKillPoll {
    Refreeze(ResilienceFreezingProcessTree),
    Pending(ResilienceKillPendingProcessTree),
    Gone(ResilienceProducersGone),
}

impl ResilienceProducersGone {
    pub fn root(&self) -> ProcessIdentity {
        self.inner.root()
    }
}

fn durable_targets(
    targets: Vec<(crate::process::LiveProcessIdentity, ProcessIdentity)>,
) -> Vec<ProcessIdentity> {
    targets.into_iter().map(|(_, durable)| durable).collect()
}
