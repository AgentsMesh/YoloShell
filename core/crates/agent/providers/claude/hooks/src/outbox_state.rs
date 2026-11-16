use super::types::{InvocationQueue, OutboxState};
use super::*;
use crate::claude_hooks::{ClaudeHookOperation, MAX_CLAUDE_OUTBOX_INVOCATIONS};
use anyhow::{Context, Result, ensure};

pub(super) fn enqueue(
    state: &mut OutboxState,
    credentials: ClaudeHookCredentials,
    event: ClaudeLifecycleEvent,
    transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
) -> Result<Option<Delivery>> {
    super::retirement::prune_dead(state);
    let queue = queue(state, credentials)?;
    let sequence = queue.enqueue(event, transcript_identity)?;
    let owner = ProcessClaim::current()?;
    let claimed = queue
        .claim
        .is_none_or(|claim| claim.relation() == crate::process::ProcessIdentityRelation::Gone);
    if claimed {
        queue.claim = Some(owner);
    }
    ensure!(
        sequence < queue.next_sequence,
        outbox_error::SEQUENCE_NOT_RESERVED
    );
    Ok(claimed.then_some(Delivery {
        invocation_id: credentials.invocation_id,
        owner,
    }))
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn enqueue_for_recovery(
    state: &mut OutboxState,
    credentials: ClaudeHookCredentials,
    event: ClaudeLifecycleEvent,
    transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
    agent: crate::process::ProcessIdentity,
) -> Result<()> {
    let queue = queue(state, credentials)?;
    ensure!(
        queue.pending.is_empty() || queue.agent.identity() == agent,
        outbox_error::AGENT_CHANGED
    );
    queue.agent = ProcessClaim::from_identity(agent);
    queue.claim = None;
    queue.enqueue(event, transcript_identity)?;
    Ok(())
}

pub(super) fn next(state: &OutboxState, delivery: Delivery) -> Result<Option<ClaudeHookRequest>> {
    let queue = state
        .invocations
        .iter()
        .find(|queue| {
            queue.credentials.invocation_id == delivery.invocation_id
                && queue.claim == Some(delivery.owner)
        })
        .context(outbox_error::DELIVERY_CLAIM_LOST)?;
    Ok(queue.pending.front().map(|queued| queued.request.clone()))
}

pub(super) fn acknowledge(
    state: &mut OutboxState,
    delivery: Delivery,
    accepted: &ClaudeHookRequest,
) -> Result<()> {
    let queue = owned_queue(state, delivery)?;
    ensure!(
        queue.pending.front().map(|queued| &queued.request) == Some(accepted),
        outbox_error::ACKNOWLEDGEMENT_ORDER
    );
    queue.acknowledged_sequence = sequence(accepted)?;
    queue.pending.pop_front();
    Ok(())
}

pub(super) fn release_if_empty(state: &mut OutboxState, delivery: Delivery) -> Result<bool> {
    let queue = owned_queue(state, delivery)?;
    if queue.pending.is_empty() {
        queue.claim = None;
        return Ok(true);
    }
    Ok(false)
}

fn queue(
    state: &mut OutboxState,
    credentials: ClaudeHookCredentials,
) -> Result<&mut InvocationQueue> {
    if let Some(index) = state
        .invocations
        .iter()
        .position(|queue| queue.credentials.invocation_id == credentials.invocation_id)
    {
        ensure!(
            state.invocations[index].credentials == credentials,
            outbox_error::CREDENTIALS_CHANGED
        );
        return Ok(&mut state.invocations[index]);
    }
    ensure!(
        state.invocations.len() < MAX_CLAUDE_OUTBOX_INVOCATIONS,
        outbox_error::INVOCATION_CAPACITY
    );
    state
        .invocations
        .push_back(InvocationQueue::new(credentials)?);
    Ok(state
        .invocations
        .back_mut()
        .expect(outbox_error::STATE_INVALID))
}

fn owned_queue(state: &mut OutboxState, delivery: Delivery) -> Result<&mut InvocationQueue> {
    state
        .invocations
        .iter_mut()
        .find(|queue| {
            queue.credentials.invocation_id == delivery.invocation_id
                && queue.claim == Some(delivery.owner)
        })
        .context(outbox_error::DELIVERY_CLAIM_LOST)
}

fn sequence(request: &ClaudeHookRequest) -> Result<u64> {
    match request.operation {
        ClaudeHookOperation::Lifecycle { event_sequence, .. } => Ok(event_sequence),
        ClaudeHookOperation::Begin { .. } => anyhow::bail!(outbox_error::BEGIN_REQUEST),
    }
}
