use super::session_start_types::{
    PreparedSessionStart, SessionStartAcceptance, SessionStartPreparation,
};
use super::types::{AuthenticatedSessionStart, InvocationQueue, OutboxState};
use super::*;
use anyhow::{Context, Result, ensure};

pub(super) fn authenticated(
    state: &OutboxState,
    credentials: ClaudeHookCredentials,
    requested: AuthenticatedSessionStart,
    agent: crate::process::ProcessIdentity,
) -> bool {
    state.invocations.iter().any(|queue| {
        queue.credentials == credentials
            && queue.agent.identity() == agent
            && queue.authenticated_session_start.is_some_and(|committed| {
                committed == requested
                    || (committed.event_sequence < requested.event_sequence
                        && event_present(queue, requested))
            })
    })
}

pub(super) fn prepare(
    state: &mut OutboxState,
    credentials: ClaudeHookCredentials,
    accepted: AuthenticatedSessionStart,
    agent: crate::process::ProcessIdentity,
    observed: crate::sha256_digest::Sha256Digest,
    planned: crate::sha256_digest::Sha256Digest,
) -> Result<SessionStartPreparation> {
    let queue = queue(state, credentials, accepted, agent)?;
    if let Some(committed) = queue.authenticated_session_start {
        if committed != accepted {
            return Ok(SessionStartPreparation::Established);
        }
        ensure!(
            queue
                .prepared_session_start
                .is_none_or(|prepared| prepared.authority_after == observed),
            outbox_error::SESSION_START_AUTHORITY_CHANGED
        );
        return Ok(SessionStartPreparation::Committed);
    }
    if let Some(prepared) = queue.prepared_session_start {
        ensure!(
            prepared.accepted == accepted,
            outbox_error::SESSION_START_TRANSACTION_CONFLICT
        );
        if observed == prepared.authority_after {
            return Ok(SessionStartPreparation::Commit);
        }
        ensure!(
            observed == prepared.authority_before && planned == prepared.authority_after,
            outbox_error::SESSION_START_AUTHORITY_CHANGED
        );
        return Ok(SessionStartPreparation::Apply);
    }
    queue.prepared_session_start = Some(PreparedSessionStart {
        accepted,
        authority_before: observed,
        authority_after: planned,
    });
    Ok(SessionStartPreparation::Apply)
}

pub(super) fn commit(
    state: &mut OutboxState,
    credentials: ClaudeHookCredentials,
    accepted: AuthenticatedSessionStart,
    agent: crate::process::ProcessIdentity,
    observed: crate::sha256_digest::Sha256Digest,
) -> Result<SessionStartAcceptance> {
    let queue = queue(state, credentials, accepted, agent)?;
    match queue.authenticated_session_start {
        Some(current) if current == accepted => return Ok(SessionStartAcceptance::Replay),
        Some(_) => return Ok(SessionStartAcceptance::Established),
        None => {}
    }
    let prepared = queue
        .prepared_session_start
        .context(outbox_error::SESSION_START_TRANSACTION_MISSING)?;
    ensure!(
        prepared.accepted == accepted && prepared.authority_after == observed,
        outbox_error::SESSION_START_AUTHORITY_CHANGED
    );
    queue.authenticated_session_start = Some(accepted);
    Ok(SessionStartAcceptance::Recorded)
}

pub(super) fn reconcile<F>(state: &mut OutboxState, mut observed: F) -> Result<()>
where
    F: FnMut(uuid::Uuid) -> Result<Option<crate::sha256_digest::Sha256Digest>>,
{
    for queue in &mut state.invocations {
        if queue.authenticated_session_start.is_some() {
            continue;
        }
        let Some(prepared) = queue.prepared_session_start else {
            continue;
        };
        match observed(queue.credentials.invocation_id)? {
            Some(current) if current == prepared.authority_after => {
                queue.authenticated_session_start = Some(prepared.accepted);
            }
            Some(current) if current == prepared.authority_before => {
                queue.prepared_session_start = None;
            }
            None => queue.prepared_session_start = None,
            Some(_) => {}
        }
    }
    Ok(())
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn authenticate(
    state: &mut OutboxState,
    credentials: ClaudeHookCredentials,
    accepted: AuthenticatedSessionStart,
    agent: crate::process::ProcessIdentity,
) -> Result<SessionStartAcceptance> {
    let queue = queue(state, credentials, accepted, agent)?;
    match queue.authenticated_session_start {
        None => {
            let digest = crate::sha256_digest::Sha256Digest::repeated(1);
            queue.prepared_session_start = Some(PreparedSessionStart {
                accepted,
                authority_before: digest,
                authority_after: digest,
            });
            queue.authenticated_session_start = Some(accepted);
            Ok(SessionStartAcceptance::Recorded)
        }
        Some(current) if current == accepted => Ok(SessionStartAcceptance::Replay),
        Some(_) => Ok(SessionStartAcceptance::Established),
    }
}

fn queue(
    state: &mut OutboxState,
    credentials: ClaudeHookCredentials,
    accepted: AuthenticatedSessionStart,
    agent: crate::process::ProcessIdentity,
) -> Result<&mut InvocationQueue> {
    let queue = state
        .invocations
        .iter_mut()
        .find(|queue| queue.credentials == credentials)
        .context(outbox_error::RECOVERY_QUEUE_MISSING)?;
    ensure!(queue.agent.identity() == agent, outbox_error::AGENT_CHANGED);
    ensure!(
        event_present(queue, accepted),
        outbox_error::ACKNOWLEDGEMENT_ORDER
    );
    Ok(queue)
}

fn event_present(queue: &InvocationQueue, accepted: AuthenticatedSessionStart) -> bool {
    queue.pending.iter().any(|queued| {
        matches!(
            queued.request.operation,
            crate::claude_hooks::ClaudeHookOperation::Lifecycle {
                event_sequence,
                event: ClaudeLifecycleEvent::SessionStart { event_id, .. },
            } if event_sequence == accepted.event_sequence && event_id == accepted.event_id
        )
    })
}
