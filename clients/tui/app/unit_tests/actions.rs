use super::fixtures::*;
use crate::{Mode, actions::handle, constants::QUEUE_CAPACITY, network::Job};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;
use yolo_hub_rpc_protocol::Command;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn terminal_picker_selects_real_terminal_and_sidebar_prefix_does_not_terminate_shell() {
    let current = session();
    let hub = current.fields().address.hub_id;
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.replace_terminals(vec![current.clone()]);
    crate::tabs::terminal_picker::open(&mut state);
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    assert!(!handle(&mut state, &jobs, hub, key(KeyCode::Enter)));
    assert_eq!(state.mode, Mode::Terminal);
    assert_eq!(state.focused_terminal(), Some(current.fields().address));
    assert!(matches!(incoming.try_recv().unwrap(), Job::Acquire { .. }));
    let prefix = Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
    handle(&mut state, &jobs, hub, prefix);
    handle(&mut state, &jobs, hub, key(KeyCode::Char('s')));
    assert_eq!(state.mode, Mode::Inventory);
    assert!(incoming.try_recv().is_err());
    assert!(handle(&mut state, &jobs, hub, key(KeyCode::Char('q'))));
}

#[test]
fn terminal_prefix_q_requests_tui_exit_without_sending_shell_input() {
    let current = session();
    let hub = current.fields().address.hub_id;
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![current]);
    crate::tabs::terminal_picker::open(&mut state);
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    handle(&mut state, &jobs, hub, key(KeyCode::Enter));
    let _ = incoming.try_recv();
    handle(
        &mut state,
        &jobs,
        hub,
        Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL)),
    );
    assert!(handle(&mut state, &jobs, hub, key(KeyCode::Char('q'))));
    assert!(incoming.try_recv().is_err());
}

#[test]
fn empty_inventory_c_creates_a_shell_from_the_absolute_current_directory() {
    let hub = yolo_hub_identity::HubId::new();
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.entry_hub = hub;
    state.tabs = yolo_tui_tab::TabCatalog::new(hub, yolo_tui_tab::LocalTabView::default());
    crate::startup::install(
        &mut state,
        hub,
        crate::StartupAction::Automatic {
            cwd: std::env::current_dir().unwrap().to_str().unwrap().into(),
        },
    );
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    assert!(!handle(&mut state, &jobs, hub, key(KeyCode::Char('c'))));
    let Job::Call {
        command: Command::Create { request },
        ..
    } = incoming.try_recv().unwrap()
    else {
        panic!("empty inventory should create a shell")
    };
    assert!(std::path::Path::new(&request.fields().cwd).is_absolute());
}

#[test]
fn destructive_action_confirmation_keeps_original_execution_fence() {
    let current = session();
    let hub = current.fields().address.hub_id;
    let original = current.fields().runtime.unwrap().target;
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![current]);
    crate::tabs::terminal_picker::open(&mut state);
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    handle(&mut state, &jobs, hub, key(KeyCode::Char('x')));
    assert_eq!(state.mode, Mode::ConfirmTerminate);
    assert!(incoming.try_recv().is_err());
    state.replace_terminals(vec![session()]);
    handle(&mut state, &jobs, hub, key(KeyCode::Enter));
    let Job::Call {
        command: Command::Terminal { command },
        ..
    } = incoming.try_recv().unwrap()
    else {
        panic!()
    };
    assert_eq!(command.selection.address, original.address);
    assert_eq!(
        command.selection.expected_runtime,
        Some(original.runtime_epoch)
    );
    assert_eq!(state.mode, Mode::Inventory);
}

#[test]
fn search_and_shell_creation_are_bounded_user_entries() {
    let current = session();
    let hub = current.fields().address.hub_id;
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.replace_terminals(vec![current]);
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    handle(&mut state, &jobs, hub, key(KeyCode::Char('/')));
    handle(&mut state, &jobs, hub, key(KeyCode::Char('z')));
    assert!(state.visible().is_empty());
    handle(&mut state, &jobs, hub, key(KeyCode::Backspace));
    assert!(!state.visible().is_empty());
    handle(&mut state, &jobs, hub, key(KeyCode::Esc));
    handle(&mut state, &jobs, hub, key(KeyCode::Char('n')));
    state.entry = SESSION_DIRECTORY.into();
    handle(&mut state, &jobs, hub, key(KeyCode::Enter));
    let Job::Call {
        command: Command::Create { request },
        ..
    } = incoming.try_recv().unwrap()
    else {
        panic!()
    };
    assert_eq!(request.fields().cwd, SESSION_DIRECTORY);
    assert_eq!(request.fields().name, None);
    assert_eq!(state.mode, Mode::Terminal);
    assert_eq!(state.creations.pending.len(), 1);
}
