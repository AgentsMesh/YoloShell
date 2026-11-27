use crate::fixtures::Fixture;
use yolo_tui_inventory::{TreeRowKind, TreeSelection};

fn labelled(fixture: &Fixture, label: &str) -> yolo_hub_rpc_protocol::HubSummary {
    yolo_hub_rpc_protocol::HubSummary {
        hub_id: fixture.hub.hub_id,
        reachable: true,
        label: Some(label.into()),
    }
}

#[test]
fn tree_orders_the_local_machine_first_and_groups_agents_beneath_it() {
    let first = Fixture::new();
    let second = Fixture::new();
    let mut terminals = first.terminals();
    terminals.extend(second.terminals());
    let mut inventory = first.inventory();
    inventory.replace(
        Some(vec![first.agent.clone(), second.agent.clone()]),
        Some(vec![labelled(&first, "alpha"), labelled(&second, "beta")]),
    );
    inventory.set_local_hub(second.hub.hub_id, Some("host".into()));
    let rows = inventory.tree_rows("", &terminals);
    assert!(matches!(
        rows[0].kind,
        TreeRowKind::Machine(hub) if hub == second.hub.hub_id
    ));
    assert!(matches!(rows[1].kind, TreeRowKind::Agent(_)));
    assert!(matches!(
        rows[2].kind,
        TreeRowKind::Machine(hub) if hub == first.hub.hub_id
    ));
    assert!(matches!(rows[3].kind, TreeRowKind::Agent(_)));
    assert_eq!(rows[0].count, 1);
}

#[test]
fn collapsing_hides_children_and_filtering_auto_expands_matches() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let mut inventory = fixture.inventory();
    inventory.replace(
        Some(vec![fixture.agent.clone()]),
        Some(vec![labelled(&fixture, "alpha")]),
    );
    inventory.toggle_machine(fixture.hub.hub_id);
    let rows = inventory.tree_rows("", &terminals);
    assert_eq!(rows.len(), 1);
    assert!(matches!(rows[0].kind, TreeRowKind::Machine(_)));
    let rows = inventory.tree_rows("alpha", &terminals);
    assert_eq!(rows.len(), 2);
    assert!(rows[0].expanded);
    let rows = inventory.tree_rows("missing", &terminals);
    assert!(rows.is_empty());
}

#[test]
fn agents_without_a_machine_summary_still_get_a_root() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let mut inventory = fixture.inventory();
    inventory.replace(Some(vec![fixture.agent.clone()]), None);
    let rows = inventory.tree_rows("", &terminals);
    assert!(matches!(rows[0].kind, TreeRowKind::Machine(hub) if hub == fixture.hub.hub_id));
    inventory
        .select_tree(
            TreeSelection::Machine(fixture.hub.hub_id),
            "",
            &terminals,
        );
    assert_eq!(inventory.selected_machine, Some(fixture.hub.hub_id));
    assert_eq!(inventory.selected_agent, None);
}
