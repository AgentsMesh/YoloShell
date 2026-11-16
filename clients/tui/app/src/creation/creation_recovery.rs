use crate::{Mode, State, constants::*, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;

const PENDING_LABEL: &str = "Checking earlier shell creations";
const BLOCKED_LABEL: &str = "Shell creation results need confirmation";
const RETRY_LABEL: &str = "check again";

pub(crate) fn flush(state: &mut State, jobs: &mpsc::Sender<Job>, now: Instant) {
    let expired = state
        .creations
        .pending
        .values()
        .filter(|pending| pending.confirmed.is_none() && pending.deadline <= now)
        .map(|pending| pending.request.clone())
        .collect::<Vec<_>>();
    for request in expired {
        state.creations.retire_at(&request, now);
    }
    crate::creation_recovery_persistence::flush(state, now);
    if state.creations.persistence_error.is_some()
        || !state.client_capability_enabled(ClientCapability::TerminalLifecycle)
    {
        return;
    }
    for request in state
        .creations
        .recovery
        .prepare(state.control_generation, now)
    {
        if let Err(error) = jobs.try_send(Job::CreationQuery(request)) {
            if let Job::CreationQuery(request) = error.into_inner() {
                state.creations.recovery.not_sent(&request, now);
            }
        }
    }
}

pub(crate) fn detail(state: &State) -> Option<String> {
    if !state.creations.recovery.is_empty()
        && !state.client_capability_enabled(ClientCapability::TerminalLifecycle)
    {
        return Some(crate::client_capabilities::TERMINAL_LIFECYCLE_UNAVAILABLE.into());
    }
    if !matches!(state.mode, Mode::Inventory | Mode::Terminal)
        || !state.connection_ready()
        || state.creations.recovery.is_empty()
    {
        return None;
    }
    let status = state.creations.recovery.status();
    let key = crate::finalization_persistence::RECOVER_STORAGE_KEY;
    Some(if status.blocked > 0 {
        format!(
            "{BLOCKED_LABEL}: {} · Ctrl+B {key}: {RETRY_LABEL}",
            status.blocked
        )
    } else {
        format!("{PENDING_LABEL}: {}", state.creations.recovery.len())
    })
}
