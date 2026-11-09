use super::{ClaudeHookCredentials, ClaudeHookRequest, ClaudeLifecycleEvent, outbox_error};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[path = "outbox_evidence.rs"]
mod evidence;
#[path = "outbox_persistence.rs"]
mod persistence;
#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../test_support/outbox_persistence_hook.rs"]
mod persistence_test_hook;
#[path = "outbox_recovery.rs"]
mod recovery;
#[path = "outbox_retirement.rs"]
mod retirement;
#[path = "outbox_session_start.rs"]
mod session_start;
#[path = "outbox_session_start_state.rs"]
mod session_start_state;
#[path = "outbox_session_start_types.rs"]
mod session_start_types;
#[path = "outbox_state.rs"]
mod state;
#[path = "outbox_types.rs"]
mod types;
#[path = "outbox_validation.rs"]
mod validation;
pub use session_start_types::{SessionStartAcceptance, SessionStartPreparation};
use types::ProcessClaim;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/outbox_boot_identity_tests.rs"]
mod boot_identity_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/outbox_capacity_tests.rs"]
mod capacity_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/outbox_claim_tests.rs"]
mod claim_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/outbox_handshake_tests.rs"]
mod handshake_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/outbox_session_end_tests.rs"]
mod session_end_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/outbox_session_start_fault_tests.rs"]
mod session_start_fault_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/outbox_session_start_tests.rs"]
mod session_start_tests;

#[derive(Clone, Copy)]
pub struct Delivery {
    invocation_id: Uuid,
    owner: ProcessClaim,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingLifecycleEvent {
    pub event_sequence: u64,
    pub event: ClaudeLifecycleEvent,
    pub transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingRecovery {
    pub agent: crate::process::ProcessIdentity,
    pub events: Vec<PendingLifecycleEvent>,
}

pub enum RecoveryProbe {
    Empty,
    Blocked,
    Pending(PendingRecovery),
}

pub fn path(settings_directory: &Path) -> Result<PathBuf> {
    let session = settings_directory
        .parent()
        .context(outbox_error::SETTINGS_PARENT_MISSING)?;
    Ok(session
        .join(super::CLAUDE_HOOK_OUTBOX_DIRECTORY)
        .join(super::CLAUDE_HOOK_OUTBOX_FILE))
}

pub fn enqueue(
    path: &Path,
    credentials: ClaudeHookCredentials,
    event: ClaudeLifecycleEvent,
) -> Result<Option<Delivery>> {
    let transcript_identity = evidence::transcript_identity(&event)?;
    persistence::update(path, |state| {
        state::enqueue(state, credentials, event, transcript_identity)
    })
}

pub fn next(path: &Path, delivery: Delivery) -> Result<Option<ClaudeHookRequest>> {
    persistence::read(path, |state| state::next(state, delivery))
}

pub fn acknowledge(path: &Path, delivery: Delivery, accepted: &ClaudeHookRequest) -> Result<()> {
    persistence::update(path, |state| state::acknowledge(state, delivery, accepted))
}

pub fn release_if_empty(path: &Path, delivery: Delivery) -> Result<bool> {
    persistence::update(path, |state| state::release_if_empty(state, delivery))
}

pub fn pending_recovery(
    path: &Path,
    credentials: ClaudeHookCredentials,
) -> Result<Option<PendingRecovery>> {
    persistence::read(path, |state| Ok(recovery::pending(state, credentials)))
}

pub fn probe_recovery(path: &Path, credentials: ClaudeHookCredentials) -> Result<RecoveryProbe> {
    persistence::read(path, |state| Ok(recovery::probe(state, credentials)))
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[doc(hidden)]
pub fn authenticate_session_start(
    path: &Path,
    credentials: ClaudeHookCredentials,
    event_sequence: u64,
    event_id: Uuid,
    agent: crate::process::ProcessIdentity,
) -> Result<SessionStartAcceptance> {
    session_start::authenticate_for_test(path, credentials, event_sequence, event_id, agent)
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[doc(hidden)]
pub fn authenticate_session_start_guarded(
    path: &Path,
    credentials: ClaudeHookCredentials,
    event_sequence: u64,
    event_id: Uuid,
    agent: crate::process::ProcessIdentity,
    establish: impl FnOnce() -> Result<()>,
) -> Result<SessionStartAcceptance> {
    establish()?;
    authenticate_session_start(path, credentials, event_sequence, event_id, agent)
}

pub use session_start::{
    commit_session_start, fence_session_start_replay, prepare_session_start,
    reconcile_prepared_session_starts,
};

pub fn fence_authenticated_session_start<T>(
    path: &Path,
    invocation_id: Uuid,
    agent: crate::process::ProcessIdentity,
    decide: impl FnOnce(bool) -> Result<T>,
) -> Result<T> {
    persistence::read(path, |state| {
        decide(recovery::has_authenticated_session_start(
            state,
            invocation_id,
            agent,
        ))
    })
}

pub fn acknowledge_recovery(
    path: &Path,
    credentials: ClaudeHookCredentials,
    expected: &PendingRecovery,
) -> Result<()> {
    persistence::update(path, |state| {
        recovery::acknowledge(state, credentials, expected)
    })
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[doc(hidden)]
pub fn enqueue_for_recovery(
    path: &Path,
    credentials: ClaudeHookCredentials,
    event: ClaudeLifecycleEvent,
    transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
    agent: crate::process::ProcessIdentity,
) -> Result<()> {
    persistence::update(path, |state| {
        state::enqueue_for_recovery(state, credentials, event, transcript_identity, agent)
    })
}
