use crate::{constants::NONMATCHING_FILTER, fixtures::Fixture};
use yolo_tui_inventory::{InventorySelection, View};

#[test]
fn tree_selection_walks_machines_and_agents_and_clamps_at_both_ends() {
    let first = Fixture::new();
    let second = Fixture::new();
    let mut terminals = first.terminals();
    terminals.extend(second.terminals());
    let mut inventory = first.inventory();
    inventory.replace(
        Some(vec![second.agent.clone(), first.agent.clone()]),
        Some(vec![second.hub.clone(), first.hub.clone()]),
    );
    let rows = inventory.tree_selections("", &terminals);
    inventory.tree_selected = None;
    inventory.move_selection(true, "", &terminals);
    assert_eq!(inventory.selected(None), Some(rows[0]));
    inventory.move_selection(false, "", &terminals);
    assert_eq!(inventory.selected(None), Some(rows[0]));
    for row in rows.iter().skip(1) {
        inventory.move_selection(true, "", &terminals);
        assert_eq!(inventory.selected(None), Some(*row));
    }
    let last = *rows.last().unwrap();
    let InventorySelection::Agent(last_agent) = last else {
        panic!()
    };
    inventory.move_selection(true, "", &terminals);
    assert_eq!(inventory.selected(None), Some(last));
    inventory.reconcile("", &terminals);
    assert_eq!(inventory.selected(None), Some(last));
    inventory.view = View::Terminals;
    inventory.move_selection(true, "", &terminals);
    assert_eq!(inventory.selected(None), None);
    assert_eq!(inventory.selected_agent, Some(last_agent));
}

#[test]
fn filtering_reconciles_the_tree_cursor_without_modifying_the_view() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let mut inventory = fixture.inventory();
    inventory.reconcile(NONMATCHING_FILTER, &terminals);
    assert_eq!(inventory.tree_selected, None);
    assert_eq!(inventory.selected_agent, None);
    assert_eq!(inventory.selected_machine, None);
    inventory.move_selection(true, NONMATCHING_FILTER, &terminals);
    assert_eq!(inventory.tree_selected, None);
    inventory.reconcile("", &terminals);
    assert_eq!(inventory.selected_machine, Some(fixture.hub.hub_id));
    assert_eq!(
        inventory.selected_agent,
        Some(fixture.agent.cursor().target())
    );
    assert_eq!(inventory.view, View::Tree);
}
