use crate::fixtures::Fixture;
use uuid::Uuid;
use yolo_hub_agent_protocol::{AgentSnapshot, ObservationCursor, FIRST_OBSERVATION_SEQUENCE};
use yolo_hub_identity::{AgentTarget, ExecutionId, HubId};
use yolo_tui_inventory::{Inventory, View};

#[test]
fn default_is_empty_agent_inventory_without_asserting_current_capabilities() {
    let inventory = Inventory::default();
    assert_eq!(inventory.view, View::Agents);
    assert_eq!(inventory.scope, None);
    assert!(inventory.agents.is_empty());
    assert!(inventory.machines.is_empty());
    assert!(!inventory.agents_supported);
    assert!(!inventory.machines_supported);
    assert!(!inventory.current);
    assert_eq!(inventory.selected_agent, None);
    assert_eq!(inventory.selected_machine, None);
}

#[test]
fn replacement_does_not_rewrite_view_scope_selection_or_freshness() {
    let fixture = Fixture::new();
    let mut inventory = fixture.inventory();
    inventory.scope = Some(HubId::new());
    let selected_agent = inventory.selected_agent;
    let selected_machine = inventory.selected_machine;
    inventory.current = false;
    inventory.replace(Some(Vec::new()), Some(Vec::new()));
    assert!(inventory.agents_supported);
    assert!(inventory.machines_supported);
    assert!(inventory.agents.is_empty());
    assert!(inventory.machines.is_empty());
    assert_eq!(inventory.selected_agent, selected_agent);
    assert_eq!(inventory.selected_machine, selected_machine);
    assert_eq!(inventory.view, View::Agents);
    assert!(inventory.scope.is_some());
    assert!(!inventory.current);
    inventory.replace(None, None);
    assert!(!inventory.agents_supported);
    assert!(!inventory.machines_supported);
    assert!(
        inventory
            .terminal_for_selected(&fixture.terminals())
            .is_none()
    );
    inventory.replace(
        Some(vec![fixture.agent.clone()]),
        Some(vec![fixture.hub.clone()]),
    );
    assert_eq!(inventory.agents.len(), 1);
    assert_eq!(inventory.machines.len(), 1);
}

#[test]
fn duplicate_stable_keys_replace_their_previous_observation() {
    let fixture = Fixture::new();
    let mut inventory = Inventory::default();
    let mut hub = fixture.hub.clone();
    hub.reachable = false;
    inventory.replace(
        Some(vec![fixture.agent.clone(), fixture.agent.clone()]),
        Some(vec![fixture.hub.clone(), hub]),
    );
    assert_eq!(inventory.agents.len(), 1);
    assert_eq!(inventory.machines.len(), 1);
    assert!(!inventory.machines[&fixture.hub.hub_id].reachable);
}

#[test]
fn details_clear_when_the_selected_agent_disappears_from_a_current_catalogue() {
    let fixture = Fixture::new();
    let mut inventory = fixture.inventory();
    let target = fixture.agent.cursor().target();
    inventory.details = Some(target);
    inventory.replace(Some(Vec::new()), Some(vec![fixture.hub.clone()]));
    assert_eq!(inventory.details, None);
}

#[test]
fn replacement_keeps_agent_runs_with_the_same_logical_address_distinct() {
    let fixture = Fixture::new();
    let first = fixture.agent.clone();
    let mut fields = first.fields().clone();
    let target = first.cursor().target();
    let second_target = AgentTarget {
        address: target.address,
        execution_id: ExecutionId::new(),
    };
    fields.cursor = ObservationCursor::new(
        second_target,
        Uuid::new_v4(),
        FIRST_OBSERVATION_SEQUENCE,
    )
    .unwrap();
    let second = AgentSnapshot::new(fields).unwrap();
    let mut inventory = Inventory::default();
    inventory.replace(Some(vec![first, second]), None);
    assert_eq!(inventory.agents.len(), 2);
    assert!(inventory.agents.contains_key(&target));
    assert!(inventory.agents.contains_key(&second_target));
    inventory.history = true;
    assert_eq!(inventory.visible_agents("", &fixture.terminals()).len(), 2);
}
