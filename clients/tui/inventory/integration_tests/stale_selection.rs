use crate::{constants::*, fixtures::Fixture, render_support::render};
use ratatui::{Terminal, backend::TestBackend};
use std::collections::BTreeMap;
use yolo_hub_identity::{AgentId, RuntimeEpoch};
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalSummary};
use yolo_tui_inventory::{HEALTH_WARNING_GLYPH, HUB_UNKNOWN_GLYPH, View, render_details};

#[test]
fn lost_entry_keeps_observation_data_but_cannot_open_its_cached_agent() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let mut inventory = fixture.inventory();
    let before = inventory.agents.clone();
    assert!(inventory.terminal_for_selected(&terminals).is_some());
    inventory.current = false;
    assert!(inventory.terminal_for_selected(&terminals).is_none());
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    for view in [View::Terminals, View::Tree] {
        inventory.view = view;
        terminal
            .draw(|frame| render(frame, &inventory, &terminals, None, "", frame.area()))
            .unwrap();
        let content = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(content.contains("Stale"));
        match view {
            View::Tree => {
                // The compact row marks stale evidence without repeating healthy
                // metadata; details retain the full observation.
                assert!(content.contains(HEALTH_WARNING_GLYPH));
                assert!(content.contains(PROVIDER_KIND));
                assert!(!content.contains("Working"));
                assert!(!content.contains(AGENT_WORKING_DIRECTORY));
            }
            View::Tree => assert!(content.contains(HUB_UNKNOWN_GLYPH)),
            View::Terminals => {}
        }
        assert!(!content.contains("Unreachable"));
    }
    inventory.view = View::Tree;
    inventory.details = Some(fixture.agent.cursor().target());
    terminal
        .draw(|frame| {
            assert!(render_details(
                frame,
                &inventory,
                &terminals,
                None,
                frame.area()
            ));
        })
        .unwrap();
    let details = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    for expected in [
        "Working",
        "Stale",
        LAST_KNOWN_DIRECTORY_LABEL,
        AGENT_WORKING_DIRECTORY,
    ] {
        assert!(details.contains(expected), "missing {expected}: {details}");
    }
    assert_eq!(inventory.agents, before);
    assert!(inventory.machines[&fixture.hub.hub_id].reachable);
    assert_eq!(inventory.visible_machines("unknown"), vec![fixture.hub.hub_id]);
}

#[test]
fn refreshed_catalogue_cannot_retarget_agent_selection_to_a_reused_terminal() {
    let fixture = Fixture::new();
    let inventory = fixture.inventory();
    let original = fixture.agent.fields().terminal;
    let mut fields = fixture.session.fields().clone();
    fields.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new();
    let terminals = std::collections::BTreeMap::from([(
        fields.address,
        TerminalSummary::new(fields.clone()).unwrap(),
    )]);
    assert!(inventory.terminal_for_selected(&terminals).is_none());
    fields.runtime.as_mut().unwrap().target = original;
    fields.agent.as_mut().unwrap().address.agent_id = AgentId::new();
    let terminals =
        std::collections::BTreeMap::from([(fields.address, TerminalSummary::new(fields).unwrap())]);
    assert!(inventory.terminal_for_selected(&terminals).is_none());
}

#[test]
fn scope_no_longer_hides_other_hub_resources() {
    let first = Fixture::new();
    let second = Fixture::new();
    let mut terminals = first.terminals();
    terminals.extend(second.terminals());
    let mut inventory = first.inventory();
    inventory.replace(
        Some(vec![first.agent.clone(), second.agent.clone()]),
        Some(vec![first.hub.clone(), second.hub.clone()]),
    );
    inventory.scope = Some(first.hub.hub_id);
    inventory.view = View::Tree;
    inventory.reconcile("", &terminals);
    assert_eq!(inventory.visible_machines("").len(), 2);
    assert_eq!(inventory.visible_agents("", &terminals).len(), 2);
    assert_eq!(inventory.visible_terminals("", &terminals).len(), 2);
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| render(frame, &inventory, &terminals, None, "", frame.area()))
        .unwrap();
    let content = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(content.contains(&first.hub.hub_id.to_string()[..8]));
    assert!(content.contains(&second.hub.hub_id.to_string()[..8]));
}

#[test]
fn live_agent_details_close_with_the_agents_last_catalogue_evidence() {
    let fixture = Fixture::new();
    let target = fixture.agent.cursor().target();
    let terminals = fixture.terminals();
    let mut inventory = fixture.inventory();
    inventory.details = Some(target);
    inventory.reconcile("", &terminals);
    assert_eq!(inventory.details, Some(target));

    let mut fields = fixture.session.fields().clone();
    fields.runtime.as_mut().unwrap().status = RuntimeStatus::Exited;
    let ended = BTreeMap::from([(fields.address, TerminalSummary::new(fields).unwrap())]);
    inventory.reconcile("", &ended);
    assert_eq!(inventory.details, None);

    inventory.details = Some(target);
    inventory.history = true;
    inventory.reconcile("", &ended);
    assert_eq!(inventory.details, Some(target));
}

#[test]
fn observed_exit_epoch_labels_history_as_ended_before_the_catalogue_catches_up() {
    let fixture = Fixture::new();
    let address = fixture.session.fields().address;
    let epoch = fixture
        .session
        .fields()
        .runtime
        .unwrap()
        .target
        .runtime_epoch;
    let terminals = fixture.terminals();
    let mut inventory = fixture.inventory();
    inventory.history = true;
    inventory.view = View::Terminals;
    inventory.observe_ended(std::collections::BTreeMap::from([(address, epoch)]));
    inventory.reconcile("", &terminals);

    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| render(frame, &inventory, &terminals, None, "", frame.area()))
        .unwrap();
    let content = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(content.contains("Ended"), "{content}");
    assert!(!content.contains("Running"), "{content}");
}
