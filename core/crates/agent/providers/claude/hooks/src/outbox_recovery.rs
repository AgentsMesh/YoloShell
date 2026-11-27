use super::types::{OutboxState, QueuedRequest};
use super::*;
use crate::claude_hooks::ClaudeHookOperation;
use anyhow::{Context, Result, ensure};

pub(super) fn pending(
    state: &OutboxState,
    credentials: ClaudeHookCredentials,
) -> Option<PendingRecovery> {
    match probe(state, credentials) {
        RecoveryProbe::Pending(pending) => Some(pending),
        RecoveryProbe::Empty | RecoveryProbe::Blocked => None,
    }
}

pub(super) fn probe(state: &OutboxState, credentials: ClaudeHookCredentials) -> RecoveryProbe {
    let Some(queue) = state
        .invocations
        .iter()
        .find(|queue| queue.credentials == credentials)
    else {
        return RecoveryProbe::Empty;
    };
    if queue.pending.is_empty() {
        return RecoveryProbe::Empty;
    }
    if !queue.recoverable() {
        return RecoveryProbe::Blocked;
    }
    RecoveryProbe::Pending(snapshot(queue))
}

pub(super) fn has_authenticated_session_start(
    state: &OutboxState,
    invocation_id: uuid::Uuid,
    agent: crate::process::ProcessIdentity,
) -> bool {
    state.invocations.iter().any(|queue| {
        queue.credentials.invocation_id == invocation_id
            && queue.agent.identity() == agent
            && queue.authenticated_session_start.is_some()
    })
}

pub(super) fn acknowledge(
    state: &mut OutboxState,
    credentials: ClaudeHookCredentials,
    expected: &PendingRecovery,
) -> Result<()> {
    let queue = state
        .invocations
        .iter_mut()
        .find(|queue| queue.credentials == credentials)
        .context(outbox_error::RECOVERY_QUEUE_MISSING)?;
    ensure!(queue.recoverable(), outbox_error::RECOVERY_OWNER);
    let actual = snapshot(queue);
    ensure!(
        actual == *expected && actual.events.len() == queue.pending.len(),
        outbox_error::RECOVERY_CHANGED
    );
    queue.acknowledged_sequence = expected
        .events
        .last()
        .context(outbox_error::RECOVERY_CHANGED)?
        .event_sequence;
    queue.pending.clear();
    queue.claim = None;
    Ok(())
}

fn snapshot(queue: &super::types::InvocationQueue) -> PendingRecovery {
    PendingRecovery {
        agent: queue.agent.identity(),
        events: queue.pending.iter().filter_map(lifecycle).collect(),
    }
}

fn lifecycle(queued: &QueuedRequest) -> Option<PendingLifecycleEvent> {
    let ClaudeHookOperation::Lifecycle {
        event_sequence,
        event,
    } = &queued.request.operation
    else {
        return None;
    };
    Some(PendingLifecycleEvent {
        event_sequence: *event_sequence,
        event: event.clone(),
        transcript_identity: queued.transcript_identity,
    })
}
