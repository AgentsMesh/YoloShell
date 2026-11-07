use crate::{
    State,
    constants::{DISCONNECTED_LABEL, QUEUE_FULL_LABEL},
};
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_rpc_protocol::Command;

const UI_EPOCH_INCREMENT: u64 = 1;

pub(crate) fn failed(state: &mut State, command: &Command) -> bool {
    let Some(intent) = state.initial_control.as_ref() else {
        return false;
    };
    let Command::Lease { request } = command else {
        return false;
    };
    if intent.acquire_request_id != Some(request.request_id) {
        return false;
    }
    crate::terminal_input::clear_pregrant_for(state, intent.address);
    state.initial_control.take();
    true
}

pub(crate) fn not_sent(state: &mut State, request: &ControlRequest) -> bool {
    let Some(intent) = state.initial_control.as_ref() else {
        return false;
    };
    let Command::Lease { request: lease } = &request.command else {
        return false;
    };
    if intent.acquire_request_id != Some(lease.request_id) {
        return false;
    }
    crate::terminal_input::clear_pregrant_for(state, intent.address);
    state.initial_control.take();
    state.status = if request.generation.is_none() {
        DISCONNECTED_LABEL.into()
    } else {
        QUEUE_FULL_LABEL.into()
    };
    true
}

pub(crate) fn user_event(state: &mut State) {
    state.ui_epoch = state.ui_epoch.saturating_add(UI_EPOCH_INCREMENT);
    if let Some((address, acquire_request_id)) = state
        .initial_control
        .as_ref()
        .map(|intent| (intent.address, intent.acquire_request_id))
    {
        if let Some(intent) = state.initial_control.as_ref() {
            intent.scope.retire();
        }
        crate::terminal_input::clear_pregrant_for(state, address);
        if acquire_request_id.is_none() {
            state.initial_control.take();
        }
    }
}

pub(crate) fn disconnected(state: &mut State) {
    if let Some(intent) = state.initial_control.take() {
        crate::terminal_input::clear_pregrant_for(state, intent.address);
        let now = std::time::Instant::now();
        if let Some(target) = intent.target
            && crate::initial_control::valid_target(state, &intent, target)
            && state.control.permits(target)
        {
            // Carry only the exact surviving input intent. The old request and
            // any queued input remain invalid after the connection changes.
            state.control.desire(target, now);
        }
        intent.scope.retire();
    }
}
