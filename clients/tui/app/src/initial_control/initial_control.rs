use crate::{State, constants::QUEUE_FULL_LABEL, creation_state::PendingCreation, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_client_channels::{RequestScope, RequestValidity};
use yolo_hub_identity::{HubId, TerminalAddress};
use yolo_hub_terminal_protocol::{LeaseGrant, LeaseMode, LeaseRequest};

#[path = "initial_control_context.rs"]
mod context;
use context::current;
pub(crate) use context::valid_target;

pub(crate) use crate::initial_control_lifecycle::{disconnected, failed, not_sent, user_event};

pub(crate) struct Intent {
    pub origin: HubId,
    pub address: TerminalAddress,
    pub target: Option<yolo_hub_identity::RuntimeTarget>,
    pub tab: Uuid,
    pub pane: Uuid,
    pub generation: Uuid,
    pub ui_epoch: u64,
    pub deadline: Instant,
    pub scope: RequestScope,
    pub validity: RequestValidity,
    pub acquire_request_id: Option<Uuid>,
}

pub(crate) fn arm(
    state: &mut State,
    pending: &PendingCreation,
    address: TerminalAddress,
    now: Instant,
) -> bool {
    if !state.client_capability_enabled(ClientCapability::TerminalControl) {
        return false;
    }
    let Some(generation) = pending.generation.filter(|value| !value.is_nil()) else {
        return false;
    };
    if pending.origin != address.hub_id
        || pending.ui_epoch != state.ui_epoch
        || state.control_generation != Some(generation)
        || !crate::creation_presentation::current(state, pending)
    {
        return false;
    }
    let scope = RequestScope::default();
    let deadline = now + crate::constants::INITIAL_CONTROL_TIMEOUT;
    let validity = scope.validity_until(deadline);
    state.initial_control = Some(Intent {
        origin: pending.origin,
        address,
        target: None,
        tab: pending.tab,
        pane: pending.pane,
        generation,
        ui_epoch: pending.ui_epoch,
        deadline,
        scope,
        validity,
        acquire_request_id: None,
    });
    // Bytes typed into the vacant pane while the creation was pending now
    // belong to its address and wait for the initial writer grant.
    crate::terminal_input::adopt_creation_pregrant(state, pending, address);
    true
}

pub(crate) fn catalog(state: &mut State, jobs: &mpsc::Sender<Job>) {
    if !state.client_capability_enabled(ClientCapability::TerminalControl) {
        disconnected(state);
        return;
    }
    let Some(mut intent) = state.initial_control.take() else {
        return;
    };
    if intent.acquire_request_id.is_some() {
        state.initial_control = Some(intent);
        return;
    }
    if Instant::now() >= intent.deadline || !current(state, &intent) {
        crate::terminal_input::clear_pregrant_for(state, intent.address);
        intent.scope.retire();
        return;
    }
    let Some(target) = state
        .terminals
        .get(&intent.address)
        .and_then(|session| session.fields().runtime)
        .map(|runtime| runtime.target)
    else {
        state.initial_control = Some(intent);
        return;
    };
    let phase = crate::terminal_lifecycle::phase(state, intent.address);
    if !phase.allows_input() {
        if matches!(
            phase,
            yolo_tui_lifecycle::TerminalPhase::Starting
                | yolo_tui_lifecycle::TerminalPhase::Unreachable
                | yolo_tui_lifecycle::TerminalPhase::Unavailable
        ) {
            state.initial_control = Some(intent);
        } else {
            intent.scope.retire();
            if let Some(hint) = phase.input_blocked_hint() {
                state.status = hint.into();
            }
        }
        return;
    }
    let request_id = Uuid::new_v4();
    let request = LeaseRequest {
        request_id,
        target,
        principal: state.principal,
        mode: LeaseMode::Acquire {},
    };
    let validity = intent.validity.clone();
    if jobs
        .try_send(Job::InitialAcquire {
            generation: intent.generation,
            origin: intent.origin,
            request,
            validity,
        })
        .is_err()
    {
        intent.scope.retire();
        state.status = QUEUE_FULL_LABEL.into();
        return;
    }
    intent.acquire_request_id = Some(request_id);
    intent.target = Some(target);
    state.mode = crate::Mode::Terminal;
    state.initial_control = Some(intent);
}

pub(crate) fn granted(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    request: &LeaseRequest,
    grant: LeaseGrant,
) -> bool {
    let matches = state.initial_control.as_ref().is_some_and(|intent| {
        intent.acquire_request_id == Some(request.request_id)
            && request.target.address == intent.address
    });
    if !matches {
        return false;
    }
    let current = state.initial_control.as_ref().is_some_and(|intent| {
        valid_target(state, intent, request.target) && grant.fields().target == request.target
    });
    let intent = state
        .initial_control
        .take()
        .expect(crate::creation_constants::PENDING_CREATION_INVARIANT);
    if current && crate::terminal_lifecycle::phase(state, intent.address).allows_input() {
        state.mode = crate::Mode::Terminal;
        state.record_grant(grant, Instant::now());
        crate::terminal_input::flush_pregrant(state, jobs, intent.address, grant);
        if state.status == crate::constants::INPUT_WAITING_LABEL
            || state.status == crate::startup_constants::STARTING_SHELL
        {
            state.status = crate::constants::CONNECTED_LABEL.into();
        }
    } else if !state.client_capability_enabled(ClientCapability::TerminalControl) {
        crate::terminal_input::clear_pregrant_for(state, intent.address);
    } else if jobs
        .try_send(Job::Call {
            generation: Some(intent.generation),
            origin: grant.fields().target.address.hub_id,
            command: yolo_hub_rpc_protocol::Command::Release { grant },
        })
        .is_err()
    {
        crate::terminal_input::clear_pregrant_for(state, intent.address);
        state.status = QUEUE_FULL_LABEL.into();
    } else {
        crate::terminal_input::clear_pregrant_for(state, intent.address);
    }
    true
}
