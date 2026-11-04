use crate::{State, constants::QUEUE_CAPACITY, network::Job};
use std::collections::VecDeque;
use tokio::sync::mpsc;
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_identity::{RuntimeTarget, TerminalAddress};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::LeaseGrant;

pub(crate) struct DeferredInput {
    pub request: ControlRequest,
    pub address: TerminalAddress,
    pub target: RuntimeTarget,
    pub grant: LeaseGrant,
    pub generation: uuid::Uuid,
}

impl DeferredInput {
    pub(crate) fn current(&self, state: &State) -> bool {
        state.inputs.scope_current(state)
            && self
                .request
                .validity
                .as_ref()
                .is_some_and(|validity| validity.is_current())
            && state.control_generation == Some(self.generation)
            && state
                .grant(self.address, std::time::Instant::now())
                .is_some_and(|grant| crate::query_state::same_writer(grant, self.grant))
            && state.focused_terminal() == Some(self.address)
            && state
                .terminals
                .get(&self.address)
                .and_then(|session| session.fields().runtime)
                .is_some_and(|runtime| runtime.target == self.target)
    }

    pub(crate) fn retire(&self) {
        if let Some(validity) = &self.request.validity {
            validity.reject();
        }
    }
}

pub(crate) fn flush(state: &mut State, permit: mpsc::Permit<'_, Job>) {
    crate::terminal_input::sync(state);
    if let Some(request) = state.inputs.waiting.take() {
        permit.send(Job::UserInput(request));
        return;
    }
    if state.inputs.dispatched_len() >= QUEUE_CAPACITY {
        return;
    }
    let Some(deferred) = state.inputs.deferred.pop_front() else {
        return;
    };
    if !deferred.current(state) {
        deferred.retire();
        state.inputs.settle_pending(request_id(&deferred.request));
        return;
    }
    permit.send(Job::UserInput(deferred.request));
}

pub(crate) fn deferred_ready(state: &State) -> bool {
    !state.inputs.deferred.is_empty()
        && state.inputs.waiting.is_none()
        && state.inputs.dispatched_len() < QUEUE_CAPACITY
}

pub(crate) fn clear_deferred(state: &mut State) {
    while let Some(deferred) = state.inputs.deferred.pop_front() {
        deferred.retire();
        state.inputs.settle_pending(request_id(&deferred.request));
    }
}

pub(crate) fn clear_deferred_for(state: &mut State, address: TerminalAddress) {
    let mut retained = VecDeque::new();
    while let Some(deferred) = state.inputs.deferred.pop_front() {
        if deferred.address == address {
            deferred.retire();
            state.inputs.settle_pending(request_id(&deferred.request));
        } else {
            retained.push_back(deferred);
        }
    }
    state.inputs.deferred = retained;
}

fn request_id(request: &ControlRequest) -> uuid::Uuid {
    let Command::Input { request } = &request.command else {
        unreachable!("deferred input must contain an input command")
    };
    request.fields().request_id
}
