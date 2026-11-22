use crate::{
    State,
    constants::{DEFERRED_INPUT_CAPACITY, INPUT_OVERFLOW_LABEL, QUEUE_CAPACITY},
    deferred_input::DeferredInput,
    network::Job,
    terminal_input_scope::Scope,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use tokio::sync::mpsc::{self, error::TrySendError};
use uuid::Uuid;
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{InputRequest, LeaseGrant};

#[path = "terminal_input/pregrant.rs"]
mod pregrant;
use pregrant::PregrantInput;
pub(crate) use pregrant::PregrantTarget;
pub(crate) use pregrant::{
    adopt_creation_pregrant, adopt_startup_creation, adopt_startup_pregrant, clear_pregrant,
    clear_pregrant_for, clear_startup_pregrant, drain_pregrant, enqueue_creation_pregrant,
    enqueue_pregrant, enqueue_startup_pregrant, flush_pregrant,
};

pub(crate) const CANCELLED_LABEL: &str = "Queued input expired or terminal control changed";
const MAX_PENDING_INPUTS: usize = QUEUE_CAPACITY + DEFERRED_INPUT_CAPACITY;
pub(crate) use crate::deferred_input::{clear_deferred, clear_deferred_for, deferred_ready, flush};

#[derive(Default)]
pub(crate) struct Inputs {
    closed: bool,
    pub(super) scope: Option<Scope>,
    pub(super) streams: BTreeMap<TerminalAddress, crate::terminal_input_stream::Stream>,
    pub(super) waiting: Option<ControlRequest>,
    pub(super) interrupted: BTreeMap<TerminalAddress, LeaseGrant>,
    pub(crate) resume_requested: BTreeSet<TerminalAddress>,
    /// At least one admitted input request for this address failed or has an
    /// uncertain outcome. A later success must not erase that lost prefix.
    pub(super) failures: BTreeSet<TerminalAddress>,
    pub pending: BTreeMap<Uuid, ControlRequest>,
    pub(crate) deferred: VecDeque<DeferredInput>,
    pub(crate) overflowed: BTreeSet<TerminalAddress>,
    pub(crate) pregrant: VecDeque<PregrantInput>,
    pub(crate) snapshot_pending: BTreeSet<TerminalAddress>,
}

include!("terminal_input/state.rs");

pub(crate) fn sync(state: &mut State) {
    let scope_invalid = state
        .inputs
        .scope
        .as_ref()
        .is_some_and(|scope| !scope.current(state));
    crate::terminal_input_authority::sync(state);
    if scope_invalid {
        if let Some(request) = state.inputs.waiting.take() {
            let _ = crate::terminal_input_events::finished(
                state,
                &request,
                Some(&Err(yolo_hub_client_channels::ControlError::Invalidated)),
            );
        }
        return;
    }
    if state.inputs.waiting.as_ref().is_some_and(|request| {
        request
            .validity
            .as_ref()
            .is_none_or(|validity| !validity.is_current())
    }) {
        if let Some(request) = state.inputs.waiting.take() {
            let _ = crate::terminal_input_events::finished(
                state,
                &request,
                Some(&Err(yolo_hub_client_channels::ControlError::Invalidated)),
            );
        }
    }
}
pub(crate) fn enqueue(state: &mut State, jobs: &mpsc::Sender<Job>, input: InputRequest) -> bool {
    sync(state);
    enqueue_current(state, jobs, input)
}

/// Admit a request after the caller has already established the writer
/// boundary. Keeping this operation free of another scope sync lets a single
/// initial-control grant flush its FIFO pregrant batch without a missing first
/// snapshot cancelling the later bytes.
pub(super) fn enqueue_current(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    input: InputRequest,
) -> bool {
    if state.inputs.is_closed() {
        return false;
    }
    let Some(generation) = state.control_generation else {
        state.status = crate::constants::READ_ONLY_LABEL.into();
        return false;
    };
    if !crate::client_capabilities::require(
        state,
        yolo_builtin_client_contract::ClientCapability::TerminalControl,
        crate::client_capabilities::TERMINAL_CONTROL_UNAVAILABLE,
    ) {
        return false;
    }
    let grant = input.fields().grant;
    let address = grant.fields().target.address;
    let Some(expires) = state.writers.get(&address).map(|writer| writer.expires) else {
        return false;
    };
    state
        .inputs
        .streams
        .entry(address)
        .or_insert_with(|| crate::terminal_input_stream::Stream::new(generation, grant, expires));
    if state.inputs.overflowed.contains(&address) {
        state.status = INPUT_OVERFLOW_LABEL.into();
        return false;
    }
    let scope = state
        .inputs
        .scope
        .take()
        .unwrap_or_else(|| Scope::new(state, generation, grant));
    let validity = scope.queue.validity_until(expires);
    state.inputs.scope = Some(scope);
    let control = ControlRequest {
        generation: Some(generation),
        destination: address.hub_id,
        command: Command::Input {
            request: input.clone(),
        },
        validity: Some(validity),
    };
    let request_id = input.fields().request_id;
    if !state.inputs.can_read()
        || state.inputs.dispatched_len() >= QUEUE_CAPACITY
        || !state.inputs.deferred.is_empty()
    {
        if state.inputs.pending.len() >= MAX_PENDING_INPUTS
            || state.inputs.unsent_len() >= DEFERRED_INPUT_CAPACITY
        {
            state.inputs.overflowed.insert(address);
            state.status = INPUT_OVERFLOW_LABEL.into();
            return false;
        }
        state.inputs.pending.insert(request_id, control.clone());
        state.inputs.deferred.push_back(DeferredInput {
            request: control,
            address,
            target: grant.fields().target,
            grant,
            generation,
        });
        return true;
    }
    let retained = control.clone();
    match jobs.try_send(Job::UserInput(control)) {
        Ok(()) => {}
        Err(TrySendError::Full(Job::UserInput(control))) => {
            // Retain the first unsent delivery before appending deferred bytes.
            state.inputs.waiting = Some(control);
        }
        Err(_) => {
            state.status = crate::constants::DISCONNECTED_LABEL.into();
            return false;
        }
    }
    state.inputs.pending.insert(request_id, retained);
    true
}
