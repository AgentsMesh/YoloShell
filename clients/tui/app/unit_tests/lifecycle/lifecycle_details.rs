use super::fixtures;
use crate::{Mode, State, actions, constants::*, network::Job};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::collections::BTreeMap;
use tokio::sync::mpsc;
use yolo_hub_agent_protocol::{AgentSnapshot, Capability, Lifecycle, UnavailabilityReason};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::HubSummary;
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalSummary};
use yolo_tui_inventory::View;

fn key(state: &mut State, jobs: &mpsc::Sender<Job>, code: KeyCode) -> bool {
    let hub = state
        .terminals
        .values()
        .next()
        .map(|session| session.fields().address.hub_id)
        .unwrap_or_else(HubId::new);
    actions::handle(
        state,
        jobs,
        hub,
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE)),
    )
}

fn state() -> State {
    let (agent, session) = fixtures::agent();
    let hub = session.fields().address.hub_id;
    let mut state = crate::client_capability_support::state();
    let request_id = uuid::Uuid::new_v4();
    state.catalog_pending = Some(request_id);
    state.control_generation = Some(uuid::Uuid::new_v4());
    let (jobs, _) = mpsc::channel(QUEUE_CAPACITY);
    crate::events::apply(
        &mut state,
        &jobs,
        crate::network::Update::Catalog(yolo_hub_client_channels::CatalogEvent {
            request_id,
            result: Ok(yolo_hub_catalog_client::Catalog {
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
fn unavailable_agent_opens_details_without_dispatching_control() {
    let mut state = state();
    let address = state.inventory.selected_agent.unwrap();
    let mut agent = state.inventory.agents[&address].fields().clone();
    agent.lifecycle = Lifecycle::Exited { exit_code: None };
    agent.activity = yolo_hub_agent_protocol::Activity::Idle;
    agent.interaction = None;
    agent.capabilities.interrupt = Capability::Unavailable {
        reason: UnavailabilityReason::NotRunning,
    };
    agent.capabilities.respond_interaction = Capability::Unavailable {
        reason: UnavailabilityReason::NotRunning,
    };
    state
        .inventory
        .agents
        .insert(address, AgentSnapshot::new(agent).unwrap());
    let terminal = state.inventory.agents[&address].fields().terminal.address;
    let mut session = state.terminals[&terminal].fields().clone();
    session.runtime.as_mut().unwrap().status = RuntimeStatus::Exited;
    state
        .terminals
        .insert(terminal, TerminalSummary::new(session).unwrap());
    state.inventory.history = true;
    state.inventory.view = View::Tree;
    state.inventory.selected_agent = Some(address);
    let (jobs, mut received) = mpsc::channel(QUEUE_CAPACITY);
    assert!(!key(&mut state, &jobs, KeyCode::Enter));
    assert_eq!(state.mode, Mode::Inventory);
    assert_eq!(state.inventory.details, Some(address));
    assert!(received.try_recv().is_err());
    assert!(state.status.contains("view terminal recovery"));
}

#[test]
fn details_r_only_navigates_to_matching_terminal_history_without_control_job() {
    let mut state = state();
    let agent = state.inventory.selected_agent.unwrap();
    let terminal = state.inventory.agents[&agent].fields().terminal.address;
    let mut replaced = state.terminals[&terminal].fields().clone();
    replaced.runtime.as_mut().unwrap().status = RuntimeStatus::Exited;
    state.terminals = BTreeMap::from([(terminal, TerminalSummary::new(replaced).unwrap())]);
    state.inventory.history = true;
    state.inventory.details = Some(agent);
    state.inventory.view = View::Tree;
    let (jobs, mut received) = mpsc::channel(QUEUE_CAPACITY);
    assert!(!key(&mut state, &jobs, KeyCode::Char('R')));
    assert_eq!(state.inventory.details, None);
    assert_eq!(state.inventory.view, View::Terminals);
    assert_eq!(state.selected, Some(terminal));
    assert!(state.inventory.history);
    assert!(received.try_recv().is_err());
    assert!(state.status.contains("R resume"));
}

#[test]
fn details_r_with_missing_terminal_keeps_details_and_esc_closes_it() {
    let mut state = state();
    let agent = state.inventory.selected_agent.unwrap();
    state.terminals.clear();
    state.inventory.history = true;
    state.inventory.details = Some(agent);
    state.inventory.view = View::Tree;
    let (jobs, mut received) = mpsc::channel(QUEUE_CAPACITY);
    assert!(!key(&mut state, &jobs, KeyCode::Char('R')));
    assert_eq!(state.inventory.details, Some(agent));
    assert!(state.status.contains("view terminal recovery"));
    assert!(received.try_recv().is_err());
    assert!(!key(&mut state, &jobs, KeyCode::Esc));
    assert_eq!(state.inventory.details, None);
}

#[test]
fn details_render_when_the_layout_has_no_terminal_pane() {
    let mut state = state();
    let agent = state.inventory.selected_agent.unwrap();
    state.inventory.details = Some(agent);
    state.inventory.view = View::Tree;
    state.mode = Mode::Inventory;
    assert!(
        state
            .tab_layout()
            .panes()
            .iter()
            .all(|pane| pane.terminal.is_none())
    );
    let mut screen = ratatui::Terminal::new(ratatui::backend::TestBackend::new(90, 25)).unwrap();
    screen
        .draw(|frame| {
            crate::draw::draw(frame, &state);
        })
        .unwrap();
    let contents: String = screen
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(contents.contains("Agent run details"));
    assert!(contents.contains("Hooks"));
}
