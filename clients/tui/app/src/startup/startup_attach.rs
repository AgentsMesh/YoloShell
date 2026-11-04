use crate::{Mode, State, initial_control::Intent, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_client_channels::RequestScope;
use yolo_hub_identity::TerminalAddress;

pub(crate) fn open(state: &mut State, jobs: &mpsc::Sender<Job>, address: TerminalAddress) {
    if crate::terminal_lifecycle::gone(state, address) {
        crate::startup::retained(state, Some(address));
        return;
    }
    if state
        .focused_terminal()
        .is_some_and(|current| current != address)
    {
        crate::tabs::open(state, jobs, address);
        return;
    }
    let focused = state.tab_layout().focused();
    if let Err(error) = state.tab_layout_mut().bind(focused, Some(address)) {
        state.status = error.to_string();
        return;
    }
    state
        .creations
        .retire_binding(state.tabs.active().id, focused);
    state.selected = Some(address);
    state.mode = Mode::Terminal;
    if !state
        .terminals
        .get(&address)
        .is_some_and(crate::startup_selection::live)
    {
        state.status = crate::constants::NO_RUNTIME_LABEL.into();
        return;
    }
    if !state.client_capability_enabled(ClientCapability::TerminalControl) {
        state.status = crate::client_capabilities::TERMINAL_CONTROL_UNAVAILABLE.into();
        return;
    }
    let Some(generation) = state.control_generation else {
        return;
    };
    let scope = RequestScope::default();
    let deadline = Instant::now() + crate::constants::INITIAL_CONTROL_TIMEOUT;
    let validity = scope.validity_until(deadline);
    state.initial_control = Some(Intent {
        origin: address.hub_id,
        address,
        target: None,
        tab: state.tabs.active().id,
        pane: focused,
        generation,
        ui_epoch: state.ui_epoch,
        deadline,
        scope,
        validity,
        acquire_request_id: None,
    });
    crate::initial_control::catalog(state, jobs);
}
