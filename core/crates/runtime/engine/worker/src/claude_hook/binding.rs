use crate::agent::AgentBinding;
use crate::agent::authority::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use crate::worker::{WorkerEvent, WorkerState, lock_recover};

#[path = "binding_commit.rs"]
mod commit;

pub(super) struct BindingCommit {
    event_type: &'static str,
    binding: AgentBinding,
}

pub(in crate::worker) fn runtime_identity(state: &WorkerState) -> AuthorityRuntimeIdentity {
    let record = lock_recover(&state.record);
    AuthorityRuntimeIdentity {
        runtime_epoch: record.runtime_epoch,
        worker: AuthorityProcessIdentity {
            pid: record.worker_pid,
            start_time: record.worker_start_time,
        },
    }
}

pub(super) fn publish(state: &WorkerState, commit: Option<BindingCommit>) {
    if let Some(commit) = commit {
        state.publish_agent_event(commit.event_type, commit.binding);
    }
}

pub(super) fn exact_commit(
    before: Option<&AgentBinding>,
    after: AgentBinding,
    force_publish: bool,
) -> Option<BindingCommit> {
    commit::if_changed(before, Some(after.clone()))
        .or_else(|| commit::unchanged(force_publish, after))
}
