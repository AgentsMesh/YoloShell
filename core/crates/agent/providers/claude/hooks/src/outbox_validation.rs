use super::outbox_error;
use super::types::{InvocationQueue, OutboxState, QueuedRequest};
use crate::claude_hooks::{
    CLAUDE_HOOK_OUTBOX_SCHEMA_VERSION, CLAUDE_HOOK_SCHEMA_VERSION, ClaudeHookOperation,
    ClaudeLifecycleEvent, INITIAL_CLAUDE_EVENT_SEQUENCE, MAX_CLAUDE_FAILED_HANDSHAKE_AUDITS,
    MAX_CLAUDE_OUTBOX_INVOCATIONS,
};
use anyhow::{Result, ensure};

impl OutboxState {
    pub(super) fn validate(&self) -> Result<()> {
        let mut ids = std::collections::HashSet::new();
        ensure!(
            self.schema_version == CLAUDE_HOOK_OUTBOX_SCHEMA_VERSION
                && self.invocations.len() <= MAX_CLAUDE_OUTBOX_INVOCATIONS
                && valid_audits(self)
                && self.invocations.iter().all(|queue| {
                    !queue.credentials.invocation_id.is_nil()
                        && ids.insert(queue.credentials.invocation_id)
                        && valid_queue(queue)
                }),
            outbox_error::STATE_INVALID
        );
        Ok(())
    }
}

fn valid_audits(state: &OutboxState) -> bool {
    state.failed_handshakes.len() <= MAX_CLAUDE_FAILED_HANDSHAKE_AUDITS
        && state.failed_handshakes.iter().all(|audit| {
            !audit.invocation_id.is_nil()
                && audit.agent.pid != 0
                && audit.agent.start_time != 0
                && audit.first_sequence >= INITIAL_CLAUDE_EVENT_SEQUENCE
                && audit.last_sequence >= audit.first_sequence
                && audit.event_count != 0
        })
}

fn valid_queue(queue: &InvocationQueue) -> bool {
    let mut expected = queue.acknowledged_sequence.checked_add(1);
    let pending_valid = queue.pending.iter().all(|queued| {
        let request = &queued.request;
        let sequence = match request.operation {
            ClaudeHookOperation::Lifecycle { event_sequence, .. } => event_sequence,
            ClaudeHookOperation::Begin { .. } => return false,
        };
        let valid = request.schema_version == CLAUDE_HOOK_SCHEMA_VERSION
            && request.credentials == queue.credentials
            && valid_identity(queued)
            && expected == Some(sequence);
        expected = sequence.checked_add(1);
        valid
    });
    queue.agent.pid != 0
        && queue.agent.start_time != 0
        && queue
            .claim
            .is_none_or(|claim| claim.pid != 0 && claim.start_time != 0)
        && valid_authenticated_start(queue)
        && valid_prepared_start(queue)
        && pending_valid
        && expected == Some(queue.next_sequence)
}

fn valid_authenticated_start(queue: &InvocationQueue) -> bool {
    let Some(accepted) = queue.authenticated_session_start else {
        return true;
    };
    if accepted.event_sequence < INITIAL_CLAUDE_EVENT_SEQUENCE
        || accepted.event_sequence >= queue.next_sequence
        || accepted.event_id.is_nil()
    {
        return false;
    }
    if accepted.event_sequence <= queue.acknowledged_sequence {
        return true;
    }
    queue.pending.iter().any(|queued| {
        matches!(
            queued.request.operation,
            ClaudeHookOperation::Lifecycle {
                event_sequence,
                event: ClaudeLifecycleEvent::SessionStart { event_id, .. },
            } if event_sequence == accepted.event_sequence && event_id == accepted.event_id
        )
    })
}

fn valid_prepared_start(queue: &InvocationQueue) -> bool {
    let Some(prepared) = queue.prepared_session_start else {
        return true;
    };
    if queue
        .authenticated_session_start
        .is_some_and(|committed| committed != prepared.accepted)
    {
        return false;
    }
    valid_start_reference(queue, prepared.accepted)
}

fn valid_start_reference(
    queue: &InvocationQueue,
    accepted: super::types::AuthenticatedSessionStart,
) -> bool {
    if accepted.event_sequence < INITIAL_CLAUDE_EVENT_SEQUENCE
        || accepted.event_sequence >= queue.next_sequence
        || accepted.event_id.is_nil()
    {
        return false;
    }
    accepted.event_sequence <= queue.acknowledged_sequence
        || queue.pending.iter().any(|queued| {
            matches!(
                queued.request.operation,
                ClaudeHookOperation::Lifecycle {
                    event_sequence,
                    event: ClaudeLifecycleEvent::SessionStart { event_id, .. },
                } if event_sequence == accepted.event_sequence && event_id == accepted.event_id
            )
        })
}

fn valid_identity(queued: &QueuedRequest) -> bool {
    matches!(
        (&queued.request.operation, queued.transcript_identity),
        (
            ClaudeHookOperation::Lifecycle {
                event: ClaudeLifecycleEvent::SessionStart { .. }
                    | ClaudeLifecycleEvent::CwdChanged { .. },
                ..
            },
            Some(_)
        ) | (
            ClaudeHookOperation::Lifecycle {
                event: ClaudeLifecycleEvent::StateChanged { .. }
                    | ClaudeLifecycleEvent::SessionEnd { .. },
                ..
            },
            None
        )
    )
}
