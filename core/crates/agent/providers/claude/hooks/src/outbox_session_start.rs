use super::session_start_types::{SessionStartAcceptance, SessionStartPreparation};
use super::types::AuthenticatedSessionStart;
use super::*;
use anyhow::Result;

pub fn fence_session_start_replay<T>(
    path: &Path,
    credentials: ClaudeHookCredentials,
    event_sequence: u64,
    event_id: uuid::Uuid,
    agent: crate::process::ProcessIdentity,
    decide: impl FnOnce(bool) -> Result<T>,
) -> Result<T> {
    persistence::read(path, |state| {
        decide(super::session_start_state::authenticated(
            state,
            credentials,
            accepted(event_sequence, event_id),
            agent,
        ))
    })
}

pub fn prepare_session_start(
    path: &Path,
    credentials: ClaudeHookCredentials,
    event_sequence: u64,
    event_id: uuid::Uuid,
    agent: crate::process::ProcessIdentity,
    observed: crate::sha256_digest::Sha256Digest,
    planned: crate::sha256_digest::Sha256Digest,
) -> Result<SessionStartPreparation> {
    persistence::update(path, |state| {
        super::session_start_state::prepare(
            state,
            credentials,
            accepted(event_sequence, event_id),
            agent,
            observed,
            planned,
        )
    })
}

pub fn commit_session_start(
    path: &Path,
    credentials: ClaudeHookCredentials,
    event_sequence: u64,
    event_id: uuid::Uuid,
    agent: crate::process::ProcessIdentity,
    observed: crate::sha256_digest::Sha256Digest,
) -> Result<SessionStartAcceptance> {
    persistence::update(path, |state| {
        super::session_start_state::commit(
            state,
            credentials,
            accepted(event_sequence, event_id),
            agent,
            observed,
        )
    })
}

pub fn reconcile_prepared_session_starts<F>(path: &Path, observed: F) -> Result<()>
where
    F: FnMut(uuid::Uuid) -> Result<Option<crate::sha256_digest::Sha256Digest>>,
{
    persistence::update(path, |state| {
        super::session_start_state::reconcile(state, observed)
    })
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn authenticate_for_test(
    path: &Path,
    credentials: ClaudeHookCredentials,
    event_sequence: u64,
    event_id: uuid::Uuid,
    agent: crate::process::ProcessIdentity,
) -> Result<SessionStartAcceptance> {
    persistence::update(path, |state| {
        super::session_start_state::authenticate(
            state,
            credentials,
            accepted(event_sequence, event_id),
            agent,
        )
    })
}

fn accepted(event_sequence: u64, event_id: uuid::Uuid) -> AuthenticatedSessionStart {
    AuthenticatedSessionStart {
        event_sequence,
        event_id,
    }
}
