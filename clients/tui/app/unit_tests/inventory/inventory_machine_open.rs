use super::{fixtures, panes};
use crate::{Mode, State};
use yolo_hub_identity::{HubId, TerminalAddress};
use yolo_hub_rpc_protocol::HubSummary;
use yolo_tui_inventory::View;
fn machine_state() -> (State, HubId, TerminalAddress) {
    let (agent, session) = fixtures::agent();
    let hub = session.fields().address.hub_id;
    let address = session.fields().address;
    let mut state = crate::client_capability_support::state();
    state.inventory.replace(Some(vec![agent]), Some(vec![HubSummary { hub_id: hub, reachable: true, label: Some("machine".into()) }]));
    state.inventory.current = true;
    state.inventory.view = View::Machines;
    state.inventory.selected_machine = Some(hub);
    state.replace_terminals(vec![session]);
    panes::repaint(&mut state);
    (state, hub, address)
}
fn open(state: &mut State) {
    let (jobs, _pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    crate::inventory_actions::open(state, &jobs);
}
#[test]
fn machine_open_returns_to_the_last_terminal() {
    let (mut state, hub, address) = machine_state();
    state.inventory.remember_terminal(address);
    open(&mut state);
    assert_eq!((state.inventory.scope, state.inventory.view, state.mode, state.focused_terminal()), (Some(hub), View::Agents, Mode::Terminal, Some(address)));
}
#[test]
fn machine_without_a_last_terminal_scopes_the_agent_list() {
    let (mut state, hub, _) = machine_state();
    open(&mut state);
    assert_eq!((state.inventory.scope, state.inventory.view, state.mode, state.focused_terminal()), (Some(hub), View::Agents, Mode::Inventory, None));
}
