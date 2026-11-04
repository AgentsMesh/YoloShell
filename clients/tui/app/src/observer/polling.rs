use crate::{State, constants::*, events::enqueue, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_rpc_protocol::Command;

pub fn renew_leases(state: &mut State, jobs: &mpsc::Sender<Job>) {
    if state.client_capability_enabled(ClientCapability::TerminalControl) {
        maintain_control(state, jobs);
    }
    let now = Instant::now();
    crate::finalization_persistence::recover_due(state, now);
    crate::creation_confirmation::flush(state, jobs, now);
    crate::creation_recovery::flush(state, jobs, now);
    crate::finalization_actions::flush(state, jobs, now);
}

fn maintain_control(state: &mut State, jobs: &mpsc::Sender<Job>) {
    release_retired_writers(state, jobs);
    release_unbound_writers(state, jobs);
    crate::control::reconcile(state, jobs);
    let renewals = state
        .writers
        .iter()
        .filter(|(address, writer)| {
            writer.renew <= Instant::now() && !state.renewal_pending(**address)
        })
        .map(|(address, writer)| (*address, writer.grant))
        .collect::<Vec<_>>();
    for (address, grant) in renewals {
        if enqueue(
            state,
            jobs,
            Job::Call {
                generation: None,
                origin: address.hub_id,
                command: Command::Renew { grant },
            },
        ) {
            state.mark_renewal_pending(address);
        }
    }
}

fn release_retired_writers(state: &mut State, jobs: &mpsc::Sender<Job>) {
    let retired = std::mem::take(&mut state.retired_writers);
    for writer in retired {
        if !enqueue(
            state,
            jobs,
            Job::Call {
                generation: None,
                origin: writer.grant.fields().target.address.hub_id,
                command: Command::Release {
                    grant: writer.grant,
                },
            },
        ) {
            state.retired_writers.push(writer);
        }
    }
}

fn release_unbound_writers(state: &mut State, jobs: &mpsc::Sender<Job>) {
    let bound = state.bound_terminals();
    let retired = state
        .writers
        .keys()
        .filter(|address| {
            !bound.contains(address)
                || !crate::terminal_lifecycle::phase(state, **address).allows_input()
        })
        .copied()
        .collect::<Vec<_>>();
    for address in retired {
        if let Some(writer) = state.retire_writer(address) {
            state.clear_renewal_pending(address);
            state.clear_last_size(address);
            enqueue(
                state,
                jobs,
                Job::Call {
                    generation: None,
                    origin: address.hub_id,
                    command: Command::Release {
                        grant: writer.grant,
                    },
                },
            );
        }
    }
}

pub(crate) fn resume_input(state: &mut State, jobs: &mpsc::Sender<Job>, take_control: bool) {
    if !crate::client_capabilities::require(
        state,
        ClientCapability::TerminalControl,
        crate::client_capabilities::TERMINAL_CONTROL_UNAVAILABLE,
    ) {
        return;
    }
    if let Some(address) = state.focused_terminal() {
        crate::terminal_input_authority::request_resume(state, address);
    }
    acquire(state, jobs, take_control);
}

pub fn acquire(state: &mut State, jobs: &mpsc::Sender<Job>, take_control: bool) {
    if !crate::client_capabilities::require(
        state,
        ClientCapability::TerminalControl,
        crate::client_capabilities::TERMINAL_CONTROL_UNAVAILABLE,
    ) {
        return;
    }
    let Some(address) = state.focused_terminal() else {
        return;
    };
    if let Some(hint) = crate::terminal_lifecycle::input_blocked_hint(state, address) {
        state.status = hint.into();
        return;
    }
    let Some(runtime) = state
        .terminals
        .get(&address)
        .and_then(|session| session.fields().runtime)
    else {
        state.status = NO_RUNTIME_LABEL.into();
        return;
    };
    if take_control {
        crate::control::takeover(state, jobs, runtime.target);
        return;
    }
    crate::control::acquire(state, jobs, runtime.target);
}
