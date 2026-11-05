use super::fixtures::{grant, session};
use crate::{
    Mode, actions, events,
    network::{Job, Update},
};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use tokio::sync::mpsc;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::ManagementError;
use yolo_tui_inventory::View;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn management_is_entry_local_and_never_turns_configuration_input_into_terminal_input() {
    let entry = HubId::new();
    let remote = HubId::new();
    let mut state = crate::client_capability_support::state();
    state.inventory.view = View::Machines;
    state.inventory.scope = Some(remote);
    state.inventory.selected_machine = Some(remote);
    state.control_generation = Some(uuid::Uuid::new_v4());
    let session = session();
    let writer = grant(&session);
    state.record_grant(writer, std::time::Instant::now());
    let (jobs, mut requests) = mpsc::channel(4);
    assert!(!actions::handle(
        &mut state,
        &jobs,
        entry,
        key(KeyCode::Char('m'))
    ));
    assert_eq!(state.mode, Mode::HubConfiguration);
    let Job::Management(operation) = requests.try_recv().unwrap() else {
        panic!()
    };
    assert_eq!(
        operation.command,
        yolo_hub_rpc_protocol::Command::GetConfiguration {}
    );
    actions::handle(
        &mut state,
        &jobs,
        entry,
        Event::Paste("terminal command\n".into()),
    );
    assert!(requests.try_recv().is_err());
    events::apply(
        &mut state,
        &jobs,
        Update::Management {
            operation,
            result: Err(ManagementError::Unavailable),
        },
    );
    assert!(state.control_generation.is_some());
    assert_eq!(
        state
            .writers
            .get(&writer.fields().target.address)
            .unwrap()
            .grant,
        writer
    );
}

#[test]
fn late_results_from_a_closed_view_cannot_complete_or_replace_the_reopened_view() {
    let entry = HubId::new();
    let mut state = crate::client_capability_support::state();
    state.inventory.view = View::Machines;
    let (jobs, mut requests) = mpsc::channel(4);
    actions::handle(&mut state, &jobs, entry, key(KeyCode::Char('m')));
    let Job::Management(old) = requests.try_recv().unwrap() else {
        panic!()
    };
    actions::handle(&mut state, &jobs, entry, key(KeyCode::Esc));
    assert!(state.hub_manager.is_none());
    assert_eq!(state.mode, Mode::Inventory);
    actions::handle(&mut state, &jobs, entry, key(KeyCode::Char('m')));
    let Job::Management(current) = requests.try_recv().unwrap() else {
        panic!()
    };
    events::apply(
        &mut state,
        &jobs,
        Update::Management {
            operation: old,
            result: Err(ManagementError::Unavailable),
        },
    );
    assert!(state.hub_manager.as_ref().unwrap().pending());
    events::apply(
        &mut state,
        &jobs,
        Update::ManagementNotSent {
            token: current.token,
        },
    );
    assert!(!state.hub_manager.as_ref().unwrap().pending());
}

#[test]
fn configure_button_is_visible_only_in_hub_inventory_and_rejects_stale_geometry() {
    let entry = HubId::new();
    let mut state = crate::client_capability_support::state();
    state.inventory.view = View::Machines;
    state.inventory.current = true;
    state.control_generation = Some(uuid::Uuid::new_v4());
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(90, 25)).unwrap();
    terminal
        .draw(|frame| state.presentation = crate::draw::draw(frame, &state))
        .unwrap();
    let bounds = state.presentation.hub_button.unwrap();
    assert_eq!(bounds.y, crate::tabs::TAB_BAR_ROWS);
    assert_eq!(bounds.height, 1);
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: bounds.x,
        row: bounds.y,
        modifiers: KeyModifiers::NONE,
    });
    assert!(crate::hub_actions::open_event(&state, &event));
    state.mode = Mode::Terminal;
    assert!(!crate::hub_actions::open_event(&state, &event));
    state.mode = Mode::Inventory;
    state.presentation.layout = None;
    assert!(!crate::hub_actions::open_event(&state, &event));
    let (jobs, mut requests) = mpsc::channel(1);
    jobs.try_send(Job::Refresh {
        request_id: uuid::Uuid::new_v4(),
    })
    .ok()
    .unwrap();
    crate::hub_actions::open(&mut state, &jobs, entry);
    assert!(!state.hub_manager.as_ref().unwrap().pending());
    requests.try_recv().unwrap();
    assert!(requests.try_recv().is_err());
}
