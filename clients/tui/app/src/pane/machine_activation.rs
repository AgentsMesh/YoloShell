use crate::{Mode, State, network::Job};
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_identity::{HubId, TerminalAddress};
/// Select a machine's workspace, creating its first local tab if needed.
pub(crate) fn select_machine_workspace(state: &mut State, hub: HubId) {
    if state.tabs.machine() != hub {
        let mut tabs = state.tab_snapshot();
        if tabs.select_machine(hub).unwrap_or(false) {
            state.replace_tabs(tabs);
            state.activate_tab_collection();
            state.selected = state.focused_terminal();
        }
    }
}
pub(crate) fn select_machine(state: &mut State, hub: HubId) {
    select_machine_workspace(state, hub);
    state.inventory.select_tree(
        yolo_tui_inventory::TreeSelection::Machine(hub),
        &state.filter,
        &state.terminals,
    );
}
/// Enter a machine workspace: focus a live terminal, or start one shell there.
pub(crate) fn activate(state: &mut State, jobs: &mpsc::Sender<Job>, hub: HubId) {
    select_machine(state, hub);
    state.sidebar_visible = true;
    state.inventory.view = yolo_tui_inventory::View::Tree;
    state.mode = Mode::Terminal;
    state.filter.clear();
    if let Some(address) = live_terminal(state, hub) {
        crate::tabs::open(state, jobs, address);
        return;
    }
    if creation_pending(state, hub) {
        state.status = crate::startup_constants::STARTING_SHELL.into();
        return;
    }
    if !machine_reachable(state, hub) {
        state.status = crate::constants::MACHINE_UNREACHABLE_LABEL.into();
        return;
    }
    if !state.client_capability_enabled(ClientCapability::TerminalLifecycle) {
        state.status = crate::client_capabilities::TERMINAL_LIFECYCLE_UNAVAILABLE.into();
        return;
    }
    let cwd = crate::shell_actions::directory(state, hub);
    if !crate::shell_actions::create(state, jobs, hub, cwd, None) {
        state.mode = Mode::Terminal;
    }
}
fn live_terminal(state: &State, hub: HubId) -> Option<TerminalAddress> {
    let live = |address: &TerminalAddress| {
        address.hub_id == hub && !crate::terminal_lifecycle::gone(state, *address)
    };
    state
        .focused_terminal()
        .filter(live)
        .or_else(|| state.inventory.last_terminal_for(hub).filter(live))
        .or_else(|| state.terminals.keys().copied().find(|address| live(address)))
}
fn creation_pending(state: &State, hub: HubId) -> bool {
    state
        .creations
        .pending
        .values()
        .any(|pending| pending.origin == hub && pending.binding_active)
        || state
            .creations
            .finalizing
            .values()
            .any(|finalizing| finalizing.record.origin == hub)
}
fn machine_reachable(state: &State, hub: HubId) -> bool {
    state.inventory.current
        && state
            .inventory
            .machines
            .get(&hub)
            .is_none_or(|summary| summary.reachable)
}
