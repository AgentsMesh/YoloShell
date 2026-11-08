use crate::{Mode, State, actions, constants::QUEUE_CAPACITY};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;
use yolo_hub_identity::HubId;

fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

#[test]
fn detach_is_owned_by_the_application_in_every_modal_without_sending_mutations() {
    for mode in [
        Mode::Directory,
        Mode::Search,
        Mode::ConfirmTerminate,
        Mode::ConfirmArchive,
        Mode::ConfirmRecreate,
        Mode::HubConfiguration,
    ] {
        let mut state = crate::client_capability_support::state();
        state.mode = mode;
        check(&mut state);
    }
    let mut state = crate::client_capability_support::state();
    crate::help::open(&mut state);
    check(&mut state);
    let mut state = crate::client_capability_support::state();
    state.tab_manager = Some(yolo_tui_tab_manager::Manager::new(&state.tabs));
    check(&mut state);
}

fn check(state: &mut State) {
    let hub = HubId::new();
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    assert!(!actions::handle(
        state,
        &jobs,
        hub,
        key(KeyCode::Char('b'), KeyModifiers::CONTROL)
    ));
    assert!(actions::handle(
        state,
        &jobs,
        hub,
        key(KeyCode::Char('d'), KeyModifiers::NONE)
    ));
    assert!(incoming.try_recv().is_err());
}

#[test]
fn ordinary_q_stays_editable_in_a_directory_form() {
    let mut state = crate::client_capability_support::state();
    state.mode = Mode::Directory;
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    assert!(!actions::handle(
        &mut state,
        &jobs,
        HubId::new(),
        key(KeyCode::Char('q'), KeyModifiers::NONE)
    ));
    assert_eq!(state.entry, "q");
    assert!(incoming.try_recv().is_err());
}
