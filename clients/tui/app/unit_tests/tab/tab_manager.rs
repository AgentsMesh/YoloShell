use super::{creation_support::Fixture, fixtures};
use crate::{Mode, network::Job, tabs::manager};
use crossterm::event::{Event, KeyCode};
use yolo_hub_rpc_protocol::Command;
use yolo_tui_tab_manager::Manager;

const NAME: &str = "Build";
const SINGLE: usize = 1;
const PAIR: usize = 2;
const CREATE_KEY: char = 'n';
const REMOVE_KEY: char = 'x';

fn key(fixture: &mut Fixture, code: KeyCode) {
    manager::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        Event::Key(code.into()),
    );
}

fn open(fixture: &mut Fixture) {
    fixture.state.tab_manager = Some(Manager::new(&fixture.state.tabs));
}

fn start_create(fixture: &mut Fixture, name: &str) {
    open(fixture);
    key(fixture, KeyCode::Char(CREATE_KEY));
    manager::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        Event::Paste(name.into()),
    );
}

#[test]
fn manager_create_starts_a_real_shell_and_preserves_the_previous_pty() {
    let mut fixture = Fixture::new();
    start_create(&mut fixture, NAME);
    assert_eq!(fixture.state.tabs.entries().len(), SINGLE);
    assert!(fixture.incoming.try_recv().is_err());
    key(&mut fixture, KeyCode::Enter);
    assert_eq!(fixture.state.tabs.entries().len(), PAIR);
    assert_eq!(fixture.state.tabs.active().name, NAME);
    assert_eq!(fixture.state.mode, Mode::Terminal);
    assert!(fixture.state.tab_manager.is_none());
    assert_eq!(fixture.original_binding(), Some(fixture.expected));
    let Job::Call {
        origin,
        command: Command::Create { request },
        ..
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!()
    };
    assert_eq!(origin, fixture.origin);
    assert_eq!(request.fields().cwd, fixtures::SESSION_DIRECTORY);
    let pending = &fixture.state.creations.pending[&request.fields().request_id];
    assert_eq!(pending.tab, fixture.state.tabs.active().id);
    assert_eq!(pending.expected, None);
    assert!(crate::creation_presentation::current(
        &fixture.state,
        pending
    ));
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn failed_creation_admission_preserves_form_layout_and_bindings_until_explicit_retry() {
    let mut fixture = Fixture::with_capacity(SINGLE);
    fixture.state.pane_view.zoomed = true;
    start_create(&mut fixture, NAME);
    let before = fixture.state.tab_snapshot();
    fixture
        .jobs
        .try_send(Job::Refresh {
            request_id: uuid::Uuid::new_v4(),
        })
        .unwrap();
    key(&mut fixture, KeyCode::Enter);
    assert!(fixture.state.tab_manager.is_some());
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert!(fixture.state.pane_view.zoomed);
    assert!(fixture.state.creations.pending.is_empty());
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Refresh { .. }
    ));
    assert!(fixture.incoming.try_recv().is_err());
    key(&mut fixture, KeyCode::Enter);
    assert_eq!(fixture.state.tabs.active().name, NAME);
    assert_eq!(fixture.original_binding(), Some(fixture.expected));
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Call {
            command: Command::Create { .. },
            ..
        }
    ));
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn conflicting_names_and_cancelled_forms_never_create_shells() {
    let mut fixture = Fixture::new();
    let name = fixture.state.tabs.active().name.clone();
    let before = fixture.state.tab_snapshot();
    start_create(&mut fixture, &name);
    key(&mut fixture, KeyCode::Enter);
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert!(fixture.state.tab_manager.is_some());
    key(&mut fixture, KeyCode::Esc);
    key(&mut fixture, KeyCode::Esc);
    assert!(fixture.state.tab_manager.is_none());
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn selecting_an_existing_tab_acquires_its_terminal_without_creating_another() {
    let mut fixture = Fixture::new();
    fixture.state.tabs = fixture.state.tab_snapshot();
    fixture.state.tabs.create(NAME).unwrap();
    fixture.state.activate_tab_collection();
    open(&mut fixture);
    key(&mut fixture, KeyCode::Up);
    key(&mut fixture, KeyCode::Enter);
    assert_eq!(fixture.state.tabs.active().id, fixture.tab);
    assert_eq!(fixture.state.focused_terminal(), Some(fixture.expected));
    assert_eq!(fixture.state.mode, Mode::Terminal);
    assert!(
        matches!(fixture.incoming.try_recv().unwrap(), Job::Acquire {
        request, ..
    } if request.target.address == fixture.expected)
    );
    assert!(fixture.incoming.try_recv().is_err());
    assert!(fixture.state.creations.pending.is_empty());
}

#[test]
fn removing_a_tab_requires_confirmation_and_keeps_its_terminal_alive() {
    let mut fixture = Fixture::new();
    fixture.state.tabs = fixture.state.tab_snapshot();
    fixture.state.tabs.create(NAME).unwrap();
    fixture.state.activate_tab_collection();
    open(&mut fixture);
    key(&mut fixture, KeyCode::Up);
    key(&mut fixture, KeyCode::Char(REMOVE_KEY));
    assert_eq!(fixture.state.tabs.entries().len(), PAIR);
    key(&mut fixture, KeyCode::Enter);
    assert_eq!(fixture.state.tabs.entries().len(), SINGLE);
    assert!(fixture.state.terminals.contains_key(&fixture.expected));
    assert!(!fixture.state.bound_terminals().contains(&fixture.expected));
    assert!(fixture.incoming.try_recv().is_err());
}
