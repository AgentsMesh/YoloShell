use super::BindingCommit;
use crate::agent::AgentBinding;
use crate::protocol::event;

pub(super) fn if_changed(
    before: Option<&AgentBinding>,
    after: Option<AgentBinding>,
) -> Option<BindingCommit> {
    let after = after?;
    if before.is_some_and(|value| equivalent(value, &after)) {
        return None;
    }
    let identity_changed = before.is_none_or(|value| identity(value) != identity(&after));
    Some(BindingCommit {
        event_type: if identity_changed {
            event::AGENT_BINDING_CHANGED
        } else {
            event::AGENT_STATUS_CHANGED
        },
        binding: after,
    })
}

pub(super) fn unchanged(force: bool, binding: AgentBinding) -> Option<BindingCommit> {
    force.then_some(BindingCommit {
        event_type: event::AGENT_STATUS_CHANGED,
        binding,
    })
}

fn identity(binding: &AgentBinding) -> (&crate::agent::AgentKind, &str, &str, &str, &str, &str) {
    (
        &binding.kind,
        &binding.provider_session_id,
        &binding.executable,
        &binding.cwd,
        &binding.evidence_path,
        &binding.data_home,
    )
}

pub(super) fn equivalent(left: &AgentBinding, right: &AgentBinding) -> bool {
    identity(left) == identity(right)
        && left.terminal_id == right.terminal_id
        && left.process_pid == right.process_pid
        && left.process_start_time == right.process_start_time
        && left.state == right.state
        && left.recovery_availability == right.recovery_availability
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../unit_tests/claude_hook/binding_commit_inline_tests.rs"]
mod tests;
