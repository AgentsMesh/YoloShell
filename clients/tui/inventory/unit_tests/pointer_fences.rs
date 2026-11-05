use super::{constants::*, pointer_support::*};
use yolo_hub_agent_protocol::{AgentSnapshot, ObservationCursor};
use yolo_hub_identity::ExecutionId;
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_tui_inventory::View;

#[test]
fn changed_filter_scope_view_selection_and_read_freshness_invalidate_the_old_screen() {
    let (mut inventory, terminals) = catalogue();
    let selected = terminals.keys().next().copied();
    let viewport = render(&inventory, &terminals, selected, "", area());
    assert!(!viewport.current(&inventory, &terminals, selected, NONMATCHING_FILTER));
    assert!(!viewport.current(&inventory, &terminals, None, ""));
    inventory.scope = terminals.keys().next().map(|address| address.hub_id);
    assert!(!viewport.current(&inventory, &terminals, selected, ""));
    inventory.scope = None;
    inventory.current = false;
    assert!(!viewport.current(&inventory, &terminals, selected, ""));
    inventory.current = true;
    inventory.view = View::Agents;
    assert!(!viewport.current(&inventory, &terminals, selected, ""));
}

#[test]
fn capability_changes_invalidate_clicks_even_when_the_visible_sessions_are_unchanged() {
    let (mut inventory, terminals) = catalogue();
    let selected = terminals.keys().next().copied();
    let agents = inventory.agents.values().cloned().collect::<Vec<_>>();
    let hubs = inventory.machines.values().cloned().collect::<Vec<_>>();
    for capabilities in [(None, Some(hubs.clone())), (Some(agents.clone()), None)] {
        let viewport = render(&inventory, &terminals, selected, "", area());
        let visible = inventory.visible_selections("", &terminals);
        inventory.replace(capabilities.0, capabilities.1);
        assert_eq!(inventory.visible_selections("", &terminals), visible);
        assert!(!viewport.current(&inventory, &terminals, selected, ""));
        let repainted = render(&inventory, &terminals, selected, "", area());
        assert!(repainted.current(&inventory, &terminals, selected, ""));
        inventory.replace(Some(agents.clone()), Some(hubs.clone()));
        assert!(!repainted.current(&inventory, &terminals, selected, ""));
    }
}

#[test]
fn same_visible_address_does_not_authorize_a_click_after_execution_or_runtime_replacement() {
    let (mut inventory, mut terminals) = catalogue();
    inventory.view = View::Agents;
    let selected = *inventory.agents.keys().next().unwrap();
    inventory.selected_agent = Some(selected);
    let viewport = render(&inventory, &terminals, None, "", area());
    let mut fields = inventory.agents[&selected].fields().clone();
    let mut target = fields.cursor.target();
    target.execution_id = ExecutionId::new();
    fields.cursor =
        ObservationCursor::new(target, fields.cursor.stream(), fields.cursor.sequence()).unwrap();
    inventory
        .agents
        .insert(selected, AgentSnapshot::new(fields).unwrap());
    assert!(!viewport.current(&inventory, &terminals, None, ""));
    inventory.view = View::Terminals;
    let selected = *terminals.keys().next().unwrap();
    let viewport = render(&inventory, &terminals, Some(selected), "", area());
    let mut fields = terminals[&selected].fields().clone();
    fields.runtime.as_mut().unwrap().target.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
    terminals.insert(selected, TerminalSummary::new(fields).unwrap());
    assert!(!viewport.current(&inventory, &terminals, Some(selected), ""));
}

#[test]
fn catalogue_reordering_and_display_metadata_change_invalidate_hit_testing() {
    let (inventory, mut terminals) = catalogue();
    let viewport = render(&inventory, &terminals, None, "", area());
    let selected = *terminals.keys().next().unwrap();
    let mut fields = terminals[&selected].fields().clone();
    fields.name = NONMATCHING_FILTER.into();
    terminals.insert(selected, TerminalSummary::new(fields).unwrap());
    assert!(!viewport.current(&inventory, &terminals, None, ""));
    let viewport = render(&inventory, &terminals, None, "", area());
    terminals.remove(&selected);
    assert!(!viewport.current(&inventory, &terminals, None, ""));
}
