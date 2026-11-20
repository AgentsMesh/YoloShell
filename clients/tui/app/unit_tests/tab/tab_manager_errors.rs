use super::creation_support::Fixture;
use crate::tabs::manager;
use crossterm::event::{Event, KeyCode};
use yolo_tui_tab::TabCatalog;
use yolo_tui_tab_manager::Manager;

const RENAME_KEY: char = 'r';
const REMOVE_KEY: char = 'x';
const RENAMED: &str = "Production logs";

fn event(fixture: &mut Fixture, event: Event) {
    manager::handle(&mut fixture.state, &fixture.jobs, fixture.origin, event);
}
fn key(fixture: &mut Fixture, key: KeyCode) {
    event(fixture, Event::Key(key.into()));
}
fn open(fixture: &mut Fixture) {
    fixture.state.tab_manager = Some(Manager::new(&fixture.state.tabs));
}

#[test]
fn manager_rename_changes_only_the_tab_name_and_invalid_empty_rename_preserves_it() {
    for valid in [false, true] {
        let mut fixture = Fixture::new();
        let layout = fixture.state.tab_layout().clone();
        let old_name = fixture.state.tabs.active().name.clone();
        open(&mut fixture);
        key(&mut fixture, KeyCode::Char(RENAME_KEY));
        for _ in old_name.chars() {
            key(&mut fixture, KeyCode::Backspace);
        }
        if valid {
            event(&mut fixture, Event::Paste(RENAMED.into()));
        }
        key(&mut fixture, KeyCode::Enter);
        assert_eq!(
            fixture.state.tabs.active().name,
            if valid { RENAMED } else { &old_name }
        );
        assert_eq!(*fixture.state.tab_layout(), layout);
        assert_eq!(fixture.state.tab_manager.is_some(), !valid);
        assert!(fixture.incoming.try_recv().is_err());
    }
}

#[test]
fn removing_last_managed_tab_clears_its_view_without_terminating_its_pty() {
    let mut fixture = Fixture::new();
    let before = fixture.state.tab_snapshot();
    open(&mut fixture);
    key(&mut fixture, KeyCode::Char(REMOVE_KEY));
    key(&mut fixture, KeyCode::Enter);
    assert!(fixture.state.tab_manager.is_none());
    assert_eq!(fixture.state.tabs.active().id, before.active().id);
    assert_eq!(fixture.state.tabs.active().name, before.active().name);
    assert!(fixture.state.focused_terminal().is_none());
    assert!(fixture.state.terminals.contains_key(&fixture.expected));
    assert_eq!(fixture.state.mode, crate::Mode::Inventory);
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn stale_manager_selection_cannot_switch_a_replaced_catalog_or_issue_a_command() {
    let mut fixture = Fixture::new();
    let stale = Manager::new(&fixture.state.tabs);
    fixture.state.tabs = TabCatalog::default();
    fixture.state.activate_tab_collection();
    let before = fixture.state.tab_snapshot();
    fixture.state.tab_manager = Some(stale);
    key(&mut fixture, KeyCode::Enter);
    assert!(fixture.state.tab_manager.is_some());
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn an_event_after_manager_closes_does_not_reopen_or_allocate_anything() {
    let mut fixture = Fixture::new();
    let before = fixture.state.tab_snapshot();
    key(&mut fixture, KeyCode::Enter);
    assert!(fixture.state.tab_manager.is_none());
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn rejected_named_creation_rolls_back_its_own_name_and_placeholder() {
    let mut fixture = Fixture::new();
    let before = fixture.state.tab_snapshot();
    open(&mut fixture);
    key(&mut fixture, KeyCode::Char('n'));
    event(&mut fixture, Event::Paste(RENAMED.into()));
    key(&mut fixture, KeyCode::Enter);
    assert_eq!(fixture.state.tabs.active().name, RENAMED);
    let crate::network::Job::Call {
        generation,
        origin,
        command,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!()
    };
    let request = yolo_hub_client_channels::ControlRequest {
        generation,
        destination: origin,
        command,
        validity: None,
    };
    crate::creation_completion::apply(
        &mut fixture.state,
        &fixture.jobs,
        &request,
        Err(yolo_hub_client_channels::ControlError::Client(
            yolo_hub_client::ClientError::Remote(
                yolo_hub_terminal_protocol::PublicError::InvalidDirectory,
            ),
        )),
    );
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert!(fixture.state.creations.pending.is_empty());
    assert!(fixture.state.creations.recovery.is_empty());
    assert!(fixture.incoming.try_recv().is_err());
}
