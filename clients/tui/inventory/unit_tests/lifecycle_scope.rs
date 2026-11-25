use crate::fixtures::Fixture;
use yolo_hub_agent_protocol::{Activity, AgentSnapshot, Lifecycle};
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalSummary};

#[test]
fn agent_exit_moves_only_the_agent_to_history_and_keeps_its_shell() {
    let fixture = Fixture::new();
    let mut inventory = fixture.inventory();
    let mut fields = fixture.agent.fields().clone();
    fields.lifecycle = Lifecycle::Exited { exit_code: None };
    fields.activity = Activity::Idle;
    inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
    assert!(
        inventory
            .visible_agents("", &fixture.terminals())
            .is_empty()
    );
    assert_eq!(
        inventory.visible_terminals("", &fixture.terminals()).len(),
        1
    );
    inventory.toggle_history();
    assert_eq!(inventory.visible_agents("", &fixture.terminals()).len(), 1);
    assert!(
        inventory
            .terminal_for_selected(&fixture.terminals())
            .is_none()
    );
}

#[test]
fn confirmed_terminal_exit_or_loss_hides_active_rows_without_discarding_history() {
    let fixture = Fixture::new();
    for status in [RuntimeStatus::Exited, RuntimeStatus::Lost] {
        let mut inventory = fixture.inventory();
        let mut fields = fixture.session.fields().clone();
        fields.runtime.as_mut().unwrap().status = status;
        let terminals = std::collections::BTreeMap::from([(
            fields.address,
            TerminalSummary::new(fields).unwrap(),
        )]);
        assert!(inventory.visible_terminals("", &terminals).is_empty());
        assert!(inventory.visible_agents("", &terminals).is_empty());
        inventory.toggle_history();
        assert_eq!(inventory.visible_terminals("", &terminals).len(), 1);
        assert_eq!(inventory.visible_agents("", &terminals).len(), 1);
        assert!(inventory.terminal_for_selected(&terminals).is_none());
    }

    let mut fields = fixture.session.fields().clone();
    fields.archived = true;
    let terminals =
        std::collections::BTreeMap::from([(fields.address, TerminalSummary::new(fields).unwrap())]);
    let mut inventory = fixture.inventory();
    assert!(inventory.visible_terminals("", &terminals).is_empty());
    inventory.toggle_history();
    assert_eq!(inventory.visible_terminals("", &terminals).len(), 1);
    assert!(inventory.terminal_for_selected(&terminals).is_none());
}

#[test]
fn disconnected_or_missing_terminal_is_unconfirmed_and_never_hidden_as_ended() {
    let fixture = Fixture::new();
    let mut inventory = fixture.inventory();
    let mut fields = fixture.session.fields().clone();
    fields.runtime.as_mut().unwrap().status = RuntimeStatus::Unreachable;
    let terminals =
        std::collections::BTreeMap::from([(fields.address, TerminalSummary::new(fields).unwrap())]);
    assert_eq!(inventory.visible_terminals("", &terminals).len(), 1);
    assert_eq!(inventory.visible_agents("", &terminals).len(), 1);
    inventory.current = false;
    assert!(inventory.visible_agents("", &Default::default()).is_empty());
    inventory.history = true;
    assert_eq!(inventory.visible_agents("", &Default::default()).len(), 1);
    assert!(inventory.terminal_for_selected(&terminals).is_none());
}
