use super::{fixtures, query_support::Fixture};
use crate::{Mode, State, actions, network::Job};
use crossterm::event::{Event, KeyCode};
use tokio::sync::mpsc;
use yolo_hub_agent_protocol::{Activity, AgentSnapshot, Lifecycle};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalAction, TerminalSummary};
use yolo_tui_inventory::View;

const RECOVERY_KEY: char = 'R';
const INPUT_KEY: char = 'y';

fn key(state: &mut State, jobs: &mpsc::Sender<Job>, hub: HubId, code: KeyCode) {
    assert!(!actions::handle(state, jobs, hub, Event::Key(code.into())));
}

#[test]
fn agent_history_recovery_targets_ended_agent_terminal_instead_of_focused_live_terminal() {
    let focused = fixtures::session();
    let focused_address = focused.fields().address;
    let (agent, terminal) = fixtures::agent();
    let target = terminal.fields().address;
    let mut fields = terminal.fields().clone();
    fields.runtime.as_mut().unwrap().status = RuntimeStatus::Exited;
    fields.recovery = Some(yolo_hub_terminal_protocol::TerminalRecoveryHint::Resume);
    let ended = TerminalSummary::new(fields).unwrap();
    let mut agent_fields = agent.fields().clone();
    let agent_target = agent_fields.cursor.target();
    agent_fields.lifecycle = Lifecycle::Exited { exit_code: None };
    agent_fields.activity = Activity::Idle;
    let ended_agent = AgentSnapshot::new(agent_fields).unwrap();
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.replace_terminals(vec![focused, ended]);
    state.bind_focused(Some(focused_address)).unwrap();
    state.inventory.agents.insert(agent_target, ended_agent);
    state.inventory.selected_agent = Some(agent_target);
    state.inventory.details = Some(agent_target);
    state.inventory.view = View::Tree;
    state.inventory.history = true;
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    key(
        &mut state,
        &jobs,
        focused_address.hub_id,
        KeyCode::Char(RECOVERY_KEY),
    );
    assert!(state.terminal_picker.is_some());
    assert_eq!(state.inventory.view, View::Terminals);
    assert!(state.inventory.history);
    assert_eq!(state.selected, Some(target));
    assert_eq!(state.focused_terminal(), Some(focused_address));
    assert!(incoming.try_recv().is_err());
    key(
        &mut state,
        &jobs,
        focused_address.hub_id,
        KeyCode::Char(RECOVERY_KEY),
    );
    assert!(matches!(incoming.try_recv().unwrap(), Job::Call {
        origin, command: Command::Terminal { command }, ..
    } if origin == target.hub_id
        && command.selection.address == target
        && command.action == TerminalAction::Resume {}));
    assert!(incoming.try_recv().is_err());
}

#[test]
fn manager_selection_from_picker_removes_overlay_before_forwarding_new_input() {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Terminal;
    crate::tabs::terminal_picker::open(&mut fixture.state);
    fixture.state.tab_manager = Some(yolo_tui_tab_manager::Manager::new(&fixture.state.tabs));
    key(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        KeyCode::Enter,
    );
    assert!(fixture.state.tab_manager.is_none());
    assert!(fixture.state.terminal_picker.is_none());
    assert_eq!(fixture.state.mode, Mode::Terminal);
    assert_eq!(
        fixture.state.focused_terminal(),
        Some(fixture.target.address)
    );
    while let Ok(job) = fixture.incoming.try_recv() {
        assert!(matches!(job, Job::Acquire { .. }));
    }
    key(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        KeyCode::Char(INPUT_KEY),
    );
    let Job::UserInput(request) = fixture.incoming.try_recv().unwrap() else {
        panic!()
    };
    let Command::Input { request } = request.command else {
        panic!()
    };
    assert_eq!(request.fields().grant.fields().target, fixture.target);
    assert_eq!(request.fields().data, INPUT_KEY.to_string().into_bytes());
    fixture.no_jobs();
}
