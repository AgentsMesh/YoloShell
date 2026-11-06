use crate::{constants::*, fixtures::Fixture};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::{AgentSnapshot, AgentWorkingDirectory, Freshness, Reachability};
use yolo_hub_identity::{AgentTarget, RuntimeEpoch};
use yolo_hub_terminal_protocol::TerminalSummary;

#[test]
fn agent_directory_search_does_not_alias_the_shell_creation_directory() {
    let fixture = Fixture::new();
    let inventory = fixture.inventory();
    let terminals = fixture.terminals();
    let target = fixture.agent.cursor().target();
    assert_eq!(
        inventory.visible_agents(AGENT_WORKING_DIRECTORY, &terminals),
        vec![target]
    );
    assert!(
        inventory
            .visible_agents(WORKING_DIRECTORY, &terminals)
            .is_empty()
    );
    assert_eq!(
        inventory.visible_terminals(WORKING_DIRECTORY, &terminals),
        vec![fixture.session.fields().address]
    );
    assert!(
        inventory
            .visible_terminals(AGENT_WORKING_DIRECTORY, &terminals)
            .is_empty()
    );
    assert!(inventory
        .visible_agents(AGENT_WORKING_DIRECTORY, &BTreeMap::new())
        .is_empty());
    let mut inventory = fixture.inventory();
    inventory.history = true;
    assert_eq!(
        inventory.visible_agents(AGENT_WORKING_DIRECTORY, &BTreeMap::new()),
        vec![target]
    );
}

#[test]
fn unknown_agent_directory_does_not_borrow_a_shell_path() {
    let fixture = Fixture::new();
    let mut fields = fixture.agent.fields().clone();
    fields.working_directory = None;
    let mut inventory = fixture.inventory();
    inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
    assert!(
        inventory
            .visible_agents(WORKING_DIRECTORY, &fixture.terminals())
            .is_empty()
    );
    assert!(
        inventory
            .visible_agents(AGENT_WORKING_DIRECTORY, &fixture.terminals())
            .is_empty()
    );
    assert_eq!(
        inventory
            .visible_agents(SESSION_NAME, &fixture.terminals())
            .len(),
        ONE_AGENT
    );
}

#[test]
fn same_path_on_different_hubs_never_changes_origin_scope() {
    let first = Fixture::new();
    let second = Fixture::new();
    let mut terminals = first.terminals();
    terminals.extend(second.terminals());
    let mut inventory = first.inventory();
    inventory.replace(Some(vec![first.agent.clone(), second.agent]), None);
    inventory.scope = Some(first.hub.hub_id);
    assert_eq!(
        inventory.visible_agents(AGENT_WORKING_DIRECTORY, &terminals),
        vec![first.agent.cursor().target()]
    );
}

#[test]
fn changed_or_stale_directory_never_redirects_agent_selection_to_a_replacement_runtime() {
    let fixture = Fixture::new();
    let runtime_target = fixture.agent.fields().terminal;
    let agent_target = fixture.agent.cursor().target();
    let mut fields = fixture.agent.fields().clone();
    fields.working_directory =
        Some(AgentWorkingDirectory::new(OTHER_AGENT_DIRECTORY.into()).unwrap());
    let mut inventory = fixture.inventory();
    inventory.replace(
        Some(vec![AgentSnapshot::new(fields.clone()).unwrap()]),
        None,
    );
    assert_eq!(
        inventory.visible_agents(OTHER_AGENT_DIRECTORY, &fixture.terminals()),
        vec![agent_target]
    );
    assert_eq!(
        inventory.terminal_for_selected(&fixture.terminals()),
        Some(runtime_target)
    );
    let mut replacement = fixture.session.fields().clone();
    replacement.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new();
    let replaced = BTreeMap::from([(
        runtime_target.address,
        TerminalSummary::new(replacement).unwrap(),
    )]);
    assert!(inventory.terminal_for_selected(&replaced).is_none());
    fields.freshness = Freshness::Stale;
    fields.reachability = Reachability::Unreachable;
    inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
    assert_eq!(
        inventory.visible_agents(OTHER_AGENT_DIRECTORY, &fixture.terminals()),
        Vec::<AgentTarget>::new()
    );
    assert!(
        inventory
            .terminal_for_selected(&fixture.terminals())
            .is_none()
    );
}

#[test]
fn directory_search_handles_literal_path_punctuation_and_unicode() {
    let fixture = Fixture::new();
    let mut fields = fixture.agent.fields().clone();
    fields.working_directory =
        Some(AgentWorkingDirectory::new(LITERAL_AGENT_DIRECTORY.into()).unwrap());
    let mut inventory = fixture.inventory();
    inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
    assert_eq!(
        inventory.visible_agents(LITERAL_AGENT_DIRECTORY, &fixture.terminals()),
        vec![fixture.agent.cursor().target()]
    );
}
