use super::startup_support::*;
use crate::{Mode, StartupAction, startup};
use crossterm::event::{Event, KeyCode, KeyEvent};
use yolo_hub_terminal_protocol::RuntimeStatus;

#[test]
fn first_launch_waits_for_catalog_and_control_then_creates_once_across_reconnects() {
    let mut f = Fixture::automatic();
    f.apply();
    f.no_jobs();
    f.state.inventory.current = true;
    f.apply();
    f.no_jobs();
    f.ready();
    f.apply();
    let request = f.create();
    assert_eq!(request.fields().cwd, DIRECTORY);
    assert!(request.fields().name.is_none());
    f.state.disconnected();
    f.ready();
    f.apply();
    f.no_jobs();
    assert_eq!(f.state.creations.pending.len(), 1);
}

#[test]
fn automatic_restores_live_binding_and_uses_acquire_without_takeover_or_creation() {
    let mut f = Fixture::automatic();
    let shell = session(f.hub, RuntimeStatus::Running);
    f.state.replace_terminals(vec![shell.clone()]);
    f.state.bind_focused(Some(shell.fields().address)).unwrap();
    f.ready();
    f.apply();
    f.acquired(&shell);
    assert_eq!(f.state.mode, Mode::Terminal);
    f.apply();
    f.no_jobs();
}

#[test]
fn automatic_routes_an_expired_saved_terminal_to_the_picker_without_creating_a_shell() {
    let mut f = Fixture::automatic();
    let missing = session(f.hub, RuntimeStatus::Exited);
    let address = missing.fields().address;
    f.state.bind_focused(Some(address)).unwrap();
    crate::startup::install(
        &mut f.state,
        f.hub,
        StartupAction::Automatic {
            cwd: DIRECTORY.into(),
        },
    );
    f.state.replace_terminals(vec![missing]);
    f.ready();
    f.apply();
    f.no_jobs();
    assert_eq!(f.state.focused_terminal(), None);
    assert_eq!(f.state.mode, Mode::Inventory);
    assert!(f.state.terminal_picker.is_some());
    assert!(!f.state.inventory.history);
    assert_eq!(f.state.status, crate::startup_constants::RETAINED_SESSION);
}

#[test]
fn recovered_unknown_creation_blocks_a_new_automatic_shell() {
    let mut f = Fixture::automatic();
    f.ready();
    f.apply();
    let request = f.create();
    f.state.creations.retire(&request);
    startup::install(
        &mut f.state,
        f.hub,
        StartupAction::Automatic {
            cwd: DIRECTORY.into(),
        },
    );
    f.apply();
    f.no_jobs();
    assert_eq!(f.state.creations.recovery.len(), 1);
}

#[test]
fn new_shell_is_explicit_even_with_live_sessions_and_preserves_requested_name() {
    let mut f = Fixture::new(StartupAction::NewShell {
        cwd: DIRECTORY.into(),
        name: Some(NAME.into()),
    });
    f.state
        .replace_terminals(vec![session(f.hub, RuntimeStatus::Running)]);
    f.ready();
    f.apply();
    assert_eq!(f.create().fields().name.as_deref(), Some(NAME));
    f.apply();
    f.no_jobs();
}

#[test]
fn user_navigation_cancels_pending_entry_while_focus_and_resize_do_not() {
    let mut f = Fixture::automatic();
    startup::user_event(&mut f.state, &Event::FocusGained);
    startup::user_event(&mut f.state, &Event::Resize(80, 24));
    f.ready();
    f.apply();
    f.create();
    let mut f = Fixture::automatic();
    startup::user_event(&mut f.state, &Event::Key(KeyEvent::from(KeyCode::Down)));
    f.ready();
    f.apply();
    f.no_jobs();
}

#[test]
fn explicit_new_shell_survives_navigation_during_startup() {
    let mut fixture = Fixture::new(StartupAction::NewShell {
        cwd: DIRECTORY.into(),
        name: Some(NAME.into()),
    });
    startup::user_event(&mut fixture.state, &Event::Key(KeyCode::Down.into()));
    fixture.ready();
    fixture.apply();
    assert_eq!(fixture.create().fields().name.as_deref(), Some(NAME));
}

#[test]
fn explicit_attach_survives_navigation_during_startup() {
    let mut fixture = Fixture::new(StartupAction::Attach { selector: None });
    let shell = session(fixture.hub, RuntimeStatus::Running);
    fixture.state.replace_terminals(vec![shell.clone()]);
    startup::user_event(&mut fixture.state, &Event::Key(KeyCode::Down.into()));
    fixture.ready();
    fixture.apply();
    fixture.acquired(&shell);
}

#[test]
fn read_only_inventory_entry_has_no_implicit_shell_mutations() {
    let mut f = Fixture::new(StartupAction::Inventory);
    f.ready();
    f.apply();
    f.no_jobs();
}

#[test]
fn terminal_picker_survives_catalogue_initialization() {
    let mut fixture = Fixture::new(StartupAction::TerminalPicker);
    assert!(fixture.state.terminal_picker.is_some());
    fixture.apply();
    assert!(fixture.state.terminal_picker.is_some());
    fixture.state.terminal_picker = None;
    fixture.ready();
    fixture.apply();
    assert!(fixture.state.terminal_picker.is_some());
    assert_eq!(fixture.state.mode, Mode::Inventory);
    fixture.no_jobs();
}

#[test]
fn explicit_attach_waits_for_a_catalogue_page_that_carries_its_target() {
    let mut fixture = Fixture::new(StartupAction::Attach { selector: None });
    fixture.ready();
    fixture.apply();
    fixture.no_jobs();
    assert_eq!(fixture.state.mode, Mode::Inventory);
    assert!(fixture.state.terminal_picker.is_some());
    assert_eq!(
        fixture.state.status,
        crate::startup_constants::SEARCHING_TERMINAL
    );
    assert!(startup::terminal_intent_pending(&fixture.state));

    let shell = session(fixture.hub, RuntimeStatus::Running);
    fixture.state.replace_terminals(vec![shell.clone()]);
    fixture.apply();
    fixture.acquired(&shell);
    assert!(!startup::terminal_intent_pending(&fixture.state));
}

#[test]
fn explicit_attach_falls_back_to_the_picker_after_the_selection_deadline() {
    let mut fixture = Fixture::new(StartupAction::Attach { selector: None });
    fixture.ready();
    fixture.state.startup.attach_deadline = Some(std::time::Instant::now());
    fixture.apply();
    assert!(fixture.state.terminal_picker.is_some());
    assert_eq!(fixture.state.mode, Mode::Inventory);
    assert!(!startup::terminal_intent_pending(&fixture.state));
    fixture.no_jobs();
}


include!("startup_picker_actions.rs");
