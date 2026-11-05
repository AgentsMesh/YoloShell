use crate::{constants::NONMATCHING_FILTER, fixtures::Fixture};
use yolo_hub_identity::HubId;
use yolo_tui_inventory::View;

#[test]
fn agent_and_hub_selections_stay_stable_and_clamp_at_both_ends() {
    let first = Fixture::new();
    let second = Fixture::new();
    let mut terminals = first.terminals();
    terminals.extend(second.terminals());
    let mut inventory = first.inventory();
    inventory.replace(
        Some(vec![second.agent.clone(), first.agent.clone()]),
        Some(vec![second.hub.clone(), first.hub.clone()]),
    );
    let agents = inventory.visible_agents("", &terminals);
    let hubs = inventory.visible_machines("");
    inventory.selected_agent = None;
    inventory.move_selection(true, "", &terminals);
    assert_eq!(inventory.selected_agent, Some(agents[0]));
    inventory.move_selection(false, "", &terminals);
    assert_eq!(inventory.selected_agent, Some(agents[0]));
    inventory.move_selection(true, "", &terminals);
    assert_eq!(inventory.selected_agent, Some(agents[1]));
    inventory.move_selection(true, "", &terminals);
    assert_eq!(inventory.selected_agent, Some(agents[1]));
    inventory.reconcile("", &terminals);
    assert_eq!(inventory.selected_agent, Some(agents[1]));
    inventory.view = View::Machines;
    inventory.selected_machine = Some(HubId::new());
    inventory.move_selection(false, "", &terminals);
    assert_eq!(inventory.selected_machine, Some(hubs[0]));
    inventory.move_selection(true, "", &terminals);
    assert_eq!(inventory.selected_machine, Some(hubs[1]));
    inventory.move_selection(true, "", &terminals);
    assert_eq!(inventory.selected_machine, Some(hubs[1]));
    inventory.move_selection(false, "", &terminals);
    assert_eq!(inventory.selected_machine, Some(hubs[0]));
    inventory.view = View::Terminals;
    inventory.move_selection(true, "", &terminals);
    assert_eq!(inventory.selected_machine, Some(hubs[0]));
    assert_eq!(inventory.selected_agent, Some(agents[1]));
}

#[test]
fn filtering_and_scope_reconcile_without_modifying_the_selected_view() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let mut inventory = fixture.inventory();
    inventory.reconcile(NONMATCHING_FILTER, &terminals);
    assert_eq!(inventory.selected_agent, None);
    assert_eq!(inventory.selected_machine, None);
    inventory.move_selection(true, NONMATCHING_FILTER, &terminals);
    assert_eq!(inventory.selected_agent, None);
    inventory.view = View::Machines;
    inventory.move_selection(false, NONMATCHING_FILTER, &terminals);
    assert_eq!(inventory.selected_machine, None);
    inventory.reconcile("", &terminals);
    assert_eq!(
        inventory.selected_agent,
        Some(fixture.agent.cursor().target())
    );
    assert_eq!(inventory.selected_machine, Some(fixture.hub.hub_id));
    inventory.scope = Some(HubId::new());
    inventory.reconcile("", &terminals);
    assert_eq!(inventory.selected_agent, None);
    assert_eq!(inventory.selected_machine, Some(fixture.hub.hub_id));
    assert_eq!(inventory.view, View::Machines);
}
