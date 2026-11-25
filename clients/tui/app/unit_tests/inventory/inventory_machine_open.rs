use super::{fixtures, panes};
use crate::{Mode, State};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use yolo_hub_client_channels::{ControlEvent, ControlRequest};
use yolo_hub_identity::{HubId, TerminalAddress};
use yolo_hub_rpc_protocol::{HubSummary, Reply, RouteCapabilities};
use yolo_tui_inventory::{TreeSelection, View};

fn machine_state() -> (State, HubId, TerminalAddress) {
    let (agent, session) = fixtures::agent();
    let hub = session.fields().address.hub_id;
    let address = session.fields().address;
    let mut state = crate::client_capability_support::state();
    state.inventory.replace(
        Some(vec![agent]),
        Some(vec![HubSummary {
            hub_id: hub,
            reachable: true,
            label: Some("machine".into()),
        }]),
    );
    state.inventory.current = true;
    state.inventory.view = View::Tree;
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.entry_hub = hub;
    state.tabs = yolo_tui_tab::TabCatalog::new(hub, yolo_tui_tab::LocalTabView::default());
    state.replace_terminals(vec![session]);
    state
        .inventory
        .select_tree(TreeSelection::Machine(hub), "", &state.terminals);
    panes::repaint(&mut state);
    (state, hub, address)
}

fn open(state: &mut State) -> tokio::sync::mpsc::Receiver<crate::network::Job> {
    let (jobs, pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    crate::inventory_actions::open(state, &jobs);
    pending
}

fn remote_state(route: Option<RouteCapabilities>) -> (State, HubId) {
    let (_, session) = fixtures::agent();
    let remote = session.fields().address.hub_id;
    let local = HubId::new();
    let mut state = crate::client_capability_support::state();
    state.inventory.replace(
        Some(Vec::new()),
        Some(vec![HubSummary {
            hub_id: remote,
            reachable: true,
            label: Some("remote".into()),
        }]),
    );
    state.inventory.current = true;
    state.inventory.set_local_hub(local, None);
    state.entry_hub = local;
    state.control_generation = Some(uuid::Uuid::new_v4());
    if let Some(route) = route {
        state.inventory.set_route_capabilities(remote, route);
    }
    state.inventory.view = View::Tree;
    state
        .inventory
        .select_tree(TreeSelection::Machine(remote), "", &state.terminals);
    (state, remote)
}

#[test]
fn machine_open_returns_to_the_last_live_terminal() {
    let (mut state, _hub, address) = machine_state();
    state.inventory.remember_terminal(address);
    let _pending = open(&mut state);
    assert_eq!(state.inventory.view, View::Tree);
    assert_eq!(state.mode, Mode::Terminal);
    assert_eq!(state.focused_terminal(), Some(address));
}

#[test]
fn machine_without_a_terminal_starts_a_shell_and_reuses_the_vacant_tab() {
    let (agent, session) = fixtures::agent();
    let hub = session.fields().address.hub_id;
    let mut state = crate::client_capability_support::state();
    state.inventory.replace(
        Some(vec![agent]),
        Some(vec![HubSummary {
            hub_id: hub,
            reachable: true,
            label: Some("machine".into()),
        }]),
    );
    state.inventory.current = true;
    state.inventory.view = View::Tree;
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.entry_hub = hub;
    state.tabs = yolo_tui_tab::TabCatalog::new(hub, yolo_tui_tab::LocalTabView::default());
    state
        .inventory
        .select_tree(TreeSelection::Machine(hub), "", &state.terminals);
    let mut pending = open(&mut state);
    let Some(crate::network::Job::Call {
        origin, command, ..
    }) = pending.try_recv().ok()
    else {
        panic!("machine activation must create a shell")
    };
    assert_eq!(origin, hub);
    assert!(matches!(
        command,
        yolo_hub_rpc_protocol::Command::Create { .. }
    ));
    assert_eq!(state.mode, Mode::Terminal);
    assert_eq!(state.tabs.machine(), hub);
    assert_eq!(state.tabs.count_for(hub), 1);
}

#[test]
fn entering_a_remote_machine_without_lifecycle_access_stays_observation_only() {
    let (mut state, _) = remote_state(Some(RouteCapabilities::default()));
    let mut pending = open(&mut state);
    assert!(pending.try_recv().is_err());
    assert_eq!(
        state.status,
        crate::constants::MACHINE_OBSERVATION_ONLY_LABEL
    );
}

#[test]
fn entering_a_remote_machine_with_unknown_access_starts_route_probe() {
    let (mut state, remote) = remote_state(None);
    let mut pending = open(&mut state);
    assert!(matches!(
        pending.try_recv().unwrap(),
        crate::network::Job::Call {
            origin,
            command: yolo_hub_rpc_protocol::Command::GetRouteCapabilities { target },
            ..
        } if origin == state.entry_hub && target == remote
    ));
    assert_eq!(
        state.status,
        crate::constants::MACHINE_ACCESS_CHECKING_LABEL
    );
}

#[test]
fn a_live_route_probe_retries_machine_activation_with_the_observed_rights() {
    let (mut state, remote) = remote_state(None);
    let (jobs, mut pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    crate::machine_activation::activate(&mut state, &jobs, remote);
    let crate::network::Job::Call {
        generation,
        origin,
        command: yolo_hub_rpc_protocol::Command::GetRouteCapabilities { target },
        ..
    } = pending.try_recv().unwrap()
    else {
        panic!("expected route probe")
    };
    crate::channel_events::apply(
        &mut state,
        &jobs,
        ControlEvent::Finished {
            request: ControlRequest {
                generation,
                destination: origin,
                command: yolo_hub_rpc_protocol::Command::GetRouteCapabilities { target },
                validity: None,
            },
            result: Ok(Reply::RouteCapabilities {
                capabilities: RouteCapabilities {
                    lifecycle: true,
                    terminal_read: true,
                    terminal_write: true,
                    agent_start: true,
                },
            }),
            connected: true,
        },
    );
    assert!(matches!(
        pending.try_recv().unwrap(),
        crate::network::Job::Call {
            origin,
            command: yolo_hub_rpc_protocol::Command::Create { .. },
            ..
        } if origin == remote
    ));
}

#[test]
fn entering_the_tree_does_not_switch_away_from_the_active_machine() {
    let (mut state, hub, _) = machine_state();
    state.mode = Mode::Inventory;
    let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    let (jobs, _pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    crate::actions::handle(&mut state, &jobs, hub, Event::Key(key));
    assert_eq!(state.tabs.machine(), hub);
}
