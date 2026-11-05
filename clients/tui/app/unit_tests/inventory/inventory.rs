use super::fixtures;
use crate::{
    Mode, State,
    actions::handle,
    events::apply,
    network::{Job, Update},
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;
use yolo_hub_catalog_client::Catalog;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, HubSummary};
use yolo_tui_inventory::View;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn catalog_state() -> State {
    let (agent, session) = fixtures::agent();
    let hub = session.fields().address.hub_id;
    let mut state = crate::client_capability_support::state();
    let request_id = uuid::Uuid::new_v4();
    state.catalog_pending = Some(request_id);
    state.control_generation = Some(uuid::Uuid::new_v4());
    let (jobs, _received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    apply(
        &mut state,
        &jobs,
        Update::Catalog(yolo_hub_client_channels::CatalogEvent {
            request_id,
            result: Ok(Catalog {
                terminals: vec![session],
                agents: Some(vec![agent]),
                hubs: Some(vec![HubSummary {
                    hub_id: hub,
                    reachable: true,
                    label: None,
                }]),
            }),
        }),
    );
    state
}

#[test]
fn agent_view_attaches_only_the_current_bound_terminal() {
    let mut state = catalog_state();
    let expected = state.selected.unwrap();
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    assert_eq!(state.inventory.view, View::Agents);
    handle(&mut state, &jobs, expected.hub_id, key(KeyCode::Enter));
    assert_eq!(state.mode, Mode::Terminal);
    assert_eq!(state.focused_terminal(), Some(expected));
    assert!(matches!(incoming.try_recv().unwrap(), Job::Acquire {
        origin, request, ..
    } if origin == expected.hub_id && request.target.address == expected));
}

#[test]
fn old_agent_binding_never_opens_a_replacement_execution() {
    let mut state = catalog_state();
    let expected = state.selected.unwrap();
    let mut session = state.terminals[&expected].fields().clone();
    session.runtime.as_mut().unwrap().target.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
    state.replace_terminals(vec![
        yolo_hub_terminal_protocol::TerminalSummary::new(session).unwrap(),
    ]);
    state.inventory.view = View::Agents;
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    handle(&mut state, &jobs, expected.hub_id, key(KeyCode::Enter));
    assert_eq!(state.mode, Mode::Inventory);
    assert!(state.focused_terminal().is_none());
    assert!(incoming.try_recv().is_err());
}

#[test]
fn failed_refresh_marks_existing_catalog_stale_without_inventing_agent_exit() {
    let mut state = catalog_state();
    let previous = state.inventory.agents.clone();
    let request_id = uuid::Uuid::new_v4();
    state.catalog_pending = Some(request_id);
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    apply(
        &mut state,
        &jobs,
        Update::Catalog(yolo_hub_client_channels::CatalogEvent {
            request_id,
            result: Err(yolo_hub_catalog_client::CatalogError::Limit),
        }),
    );
    assert!(!state.inventory.current);
    assert_eq!(state.inventory.agents, previous);
    state.inventory.view = View::Agents;
    handle(&mut state, &jobs, HubId::new(), key(KeyCode::Enter));
    assert!(incoming.try_recv().is_err());
}

#[test]
fn hub_scope_controls_shell_creation_destination_and_escape_restores_all_hubs() {
    let mut state = catalog_state();
    let expected = state.selected.unwrap().hub_id;
    let entry = HubId::new();
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    handle(&mut state, &jobs, entry, key(KeyCode::BackTab));
    assert_eq!(state.inventory.view, View::Machines);
    handle(&mut state, &jobs, entry, key(KeyCode::Char('n')));
    assert_eq!(state.mode, Mode::Directory);
    handle(&mut state, &jobs, entry, key(KeyCode::Enter));
    assert!(matches!(incoming.try_recv().unwrap(), Job::Call {
        origin, command: Command::Create { .. }, ..
    } if origin == expected));
    assert_eq!(state.mode, Mode::Terminal);
    crate::tabs::sidebar(&mut state);
    handle(&mut state, &jobs, entry, key(KeyCode::Esc));
    assert!(state.inventory.scope.is_none());
}

#[test]
fn agent_view_cannot_silently_terminate_its_shell_as_a_semantic_agent_command() {
    let mut state = catalog_state();
    state.inventory.view = View::Agents;
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    for code in ['x', 'a', 'R'] {
        handle(&mut state, &jobs, HubId::new(), key(KeyCode::Char(code)));
        assert_eq!(state.mode, Mode::Inventory);
        assert!(state.confirmation.is_none());
    }
    assert!(incoming.try_recv().is_err());
}
