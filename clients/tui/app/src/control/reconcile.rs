use crate::{State, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_client_channels::RequestScope;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_terminal_protocol::{LeaseMode, LeaseRequest};
use yolo_tui_terminal_control::{ATTEMPT_TIMEOUT, Failure};

pub(crate) fn acquire(state: &mut State, jobs: &mpsc::Sender<Job>, target: RuntimeTarget) {
    if !state.client_capability_enabled(ClientCapability::TerminalControl) {
        return;
    }
    if crate::terminal_input_authority::blocked(state, target.address) {
        return;
    }
    let entry = state.control.entry(target);
    entry.probe = None;
    if entry.takeover.is_some()
        || entry
            .scope
            .as_ref()
            .is_some_and(|(_, mode)| matches!(mode, LeaseMode::TakeControl { .. }))
    {
        entry.policy.observe();
        entry.scope = None;
    }
    entry.policy.request_write(Instant::now());
    entry.takeover = None;
    reconcile(state, jobs);
}

pub(crate) fn reconcile(state: &mut State, jobs: &mpsc::Sender<Job>) {
    if !state.client_capability_enabled(ClientCapability::TerminalControl) {
        return;
    }
    let now = Instant::now();
    let bound = state.bound_terminals();
    state.control.entries.retain(|address, entry| {
        state
            .terminals
            .get(address)
            .and_then(|terminal| terminal.fields().runtime)
            .is_some_and(|runtime| runtime.target == entry.policy.target())
    });
    let candidates = state.control.entries.keys().copied().collect::<Vec<_>>();
    for address in candidates {
        if !bound.contains(&address) {
            let entry = state.control.entries.get_mut(&address).unwrap();
            entry.policy.observe();
            entry.scope = None;
            entry.takeover = None;
        }
        let has_grant = state.grant(address, now).is_some();
        let eligible = bound.contains(&address)
            && state.control_generation.is_some()
            && !crate::terminal_input_authority::blocked(state, address)
            && !state.is_offline(address)
            && crate::terminal_lifecycle::phase(state, address).allows_input();
        let entry = state.control.entries.get_mut(&address).unwrap();
        if entry
            .takeover
            .as_ref()
            .is_some_and(|(_, deadline)| now >= *deadline)
        {
            entry.takeover = None;
            entry.policy.record_failure(Failure::Transient, now);
        }
        if entry.takeover.is_some() {
            continue;
        }
        if entry.policy.due(now, eligible, has_grant) {
            entry.scope = None;
            let target = entry.policy.target();
            dispatch(state, jobs, target, LeaseMode::Acquire {});
        }
    }
}

pub(crate) fn dispatch(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    target: RuntimeTarget,
    mode: LeaseMode,
) {
    if !state.client_capability_enabled(ClientCapability::TerminalControl) {
        return;
    }
    if crate::constants::is_self_terminal(target.address.terminal_id.as_uuid()) { return; }
    let Some(generation) = state.control_generation else {
        return;
    };
    let now = Instant::now();
    let entry = state.control.entry(target);
    entry.probe = None;
    entry.policy.request_write(now);
    let Some(attempt) = entry.policy.begin(Uuid::new_v4(), generation, now) else {
        return;
    };
    let scope = RequestScope::default();
    let validity = scope.validity_until(now + ATTEMPT_TIMEOUT);
    let request = LeaseRequest {
        request_id: attempt.request_id,
        target,
        principal: state.principal,
        mode,
    };
    if jobs
        .try_send(Job::Acquire {
            generation,
            origin: target.address.hub_id,
            request,
            validity,
        })
        .is_ok()
    {
        entry.scope = Some((scope, mode));
    } else {
        entry.policy.failed(attempt, Failure::Transient, now);
        state.status = crate::constants::QUEUE_FULL_LABEL.into();
    }
}
