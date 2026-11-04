use super::fixtures::{
    self, AGENT_WORKING_DIRECTORY, NEXT_AGENT_WORKING_DIRECTORY, SESSION_DIRECTORY,
};
use crate::{Mode, State, catalog_events, inventory_actions};
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AgentSnapshot, AgentWorkingDirectory, FIRST_OBSERVATION_SEQUENCE, ObservationCursor,
};
use yolo_hub_catalog_client::{Catalog, CatalogError};
use yolo_hub_client_channels::CatalogEvent;
use yolo_hub_terminal_protocol::TerminalSummary;

fn observed(agent: &AgentSnapshot, path: &str) -> AgentSnapshot {
    let mut fields = agent.fields().clone();
    fields.working_directory = Some(AgentWorkingDirectory::new(path.into()).unwrap());
    fields.cursor = ObservationCursor::new(
        agent.cursor().target(),
        agent.cursor().stream(),
        agent.cursor().sequence() + FIRST_OBSERVATION_SEQUENCE,
    )
    .unwrap();
    AgentSnapshot::new(fields).unwrap()
}

fn publish(state: &mut State, agent: AgentSnapshot, session: TerminalSummary) {
    let request_id = Uuid::new_v4();
    state.catalog_pending = Some(request_id);
    catalog_events::apply(state, publication(request_id, agent, session));
}

fn publication(request_id: Uuid, agent: AgentSnapshot, session: TerminalSummary) -> CatalogEvent {
    CatalogEvent {
        request_id,
        result: Ok(Catalog {
            terminals: vec![session],
            agents: Some(vec![agent]),
            hubs: None,
        }),
    }
}

#[test]
fn public_directory_update_preserves_shell_directory_and_exact_terminal_binding() {
    let (agent, session) = fixtures::agent();
    let mut state = crate::client_capability_support::state();
    let agent = observed(&agent, AGENT_WORKING_DIRECTORY);
    let target = agent.cursor().target();
    publish(&mut state, agent.clone(), session.clone());
    assert_eq!(
        state.inventory.terminal_for_selected(&state.terminals),
        Some(agent.fields().terminal)
    );
    let next = observed(&agent, NEXT_AGENT_WORKING_DIRECTORY);
    publish(&mut state, next.clone(), session.clone());
    assert_eq!(state.inventory.agents[&target], next);
    assert_eq!(state.inventory.agents[&target].cursor().target(), target);
    assert_eq!(
        state.terminals[&session.fields().address].fields().cwd,
        SESSION_DIRECTORY
    );
    assert_eq!(
        state
            .inventory
            .visible_agents(NEXT_AGENT_WORKING_DIRECTORY, &state.terminals),
        vec![target]
    );
    assert!(
        state
            .inventory
            .visible_agents(AGENT_WORKING_DIRECTORY, &state.terminals)
            .is_empty()
    );
}

#[test]
fn opening_a_selected_agent_row_switches_to_its_terminal() {
    let (agent, session) = fixtures::agent();
    let target = agent.cursor().target();
    let address = session.fields().address;
    let mut state = crate::client_capability_support::state();
    publish(&mut state, agent, session);
    state.inventory.view = yolo_tui_inventory::View::Agents;
    state.inventory.selected_agent = Some(target);
    state.mode = Mode::Inventory;
    let (jobs, _incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    inventory_actions::handle(&mut state, &jobs, crossterm::event::KeyCode::Enter);
    assert_eq!(state.focused_terminal(), Some(address));
    assert_eq!(state.mode, Mode::Terminal);
    assert_eq!(state.selected, Some(address));
}

#[test]
fn stale_refresh_cannot_restore_a_previous_directory_and_failure_keeps_last_known_path() {
    let (agent, session) = fixtures::agent();
    let previous = observed(&agent, AGENT_WORKING_DIRECTORY);
    let current = observed(&agent, NEXT_AGENT_WORKING_DIRECTORY);
    let old = Uuid::new_v4();
    let mut state = crate::client_capability_support::state();
    publish(&mut state, current.clone(), session.clone());
    catalog_events::apply(&mut state, publication(old, previous, session));
    assert_eq!(state.inventory.agents[&agent.cursor().target()], current);
    let request_id = Uuid::new_v4();
    state.catalog_pending = Some(request_id);
    catalog_events::apply(
        &mut state,
        CatalogEvent {
            request_id,
            result: Err(CatalogError::Limit),
        },
    );
    assert!(!state.inventory.current);
    assert_eq!(state.inventory.agents[&agent.cursor().target()], current);
    assert!(
        state
            .inventory
            .terminal_for_selected(&state.terminals)
            .is_none()
    );
    assert_eq!(
        state.terminals.values().next().unwrap().fields().cwd,
        SESSION_DIRECTORY
    );
}
