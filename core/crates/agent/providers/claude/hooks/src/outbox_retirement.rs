use super::types::{FailedHandshakeAudit, InvocationQueue, OutboxState};
use crate::claude_hooks::ClaudeHookOperation;

pub(super) fn prune_dead(state: &mut OutboxState) {
    let mut retained = std::collections::VecDeque::new();
    while let Some(queue) = state.invocations.pop_front() {
        let agent_gone = queue.agent.relation() == crate::process::ProcessIdentityRelation::Gone;
        let claim_gone = queue
            .claim
            .is_none_or(|claim| claim.relation() == crate::process::ProcessIdentityRelation::Gone);
        if agent_gone && claim_gone && queue.authenticated_session_start.is_none() {
            audit_failed_handshake(state, &queue);
        } else if !queue.pending.is_empty() || !agent_gone {
            retained.push_back(queue);
        }
    }
    state.invocations = retained;
}

fn audit_failed_handshake(state: &mut OutboxState, queue: &InvocationQueue) {
    let Some(first) = queue.pending.front() else {
        return;
    };
    let Some(last) = queue.pending.back() else {
        return;
    };
    if state.failed_handshakes.len() == crate::claude_hooks::MAX_CLAUDE_FAILED_HANDSHAKE_AUDITS {
        state.failed_handshakes.pop_front();
    }
    state.failed_handshakes.push_back(FailedHandshakeAudit {
        invocation_id: queue.credentials.invocation_id,
        agent: queue.agent,
        first_sequence: sequence(&first.request),
        last_sequence: sequence(&last.request),
        event_count: queue.pending.len(),
    });
}

fn sequence(request: &crate::claude_hooks::ClaudeHookRequest) -> u64 {
    match request.operation {
        ClaudeHookOperation::Lifecycle { event_sequence, .. } => event_sequence,
        ClaudeHookOperation::Begin { .. } => {
            unreachable!("{}", crate::claude_hooks::outbox_error::BEGIN_REQUEST)
        }
    }
}
