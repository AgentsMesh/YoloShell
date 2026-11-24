use super::fixtures;
use crate::{Mode, actions::handle, constants::QUEUE_CAPACITY, network::Job};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;
use yolo_hub_rpc_protocol::Command;

const DIRECTORY: &str = "/tmp/中文 directory";
const FILTER: &str = "no-matching-终端";

fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

#[test]
fn directory_paste_is_edited_locally_and_enter_submits_the_exact_value_once() {
    let current = fixtures::session();
    let hub = current.fields().address.hub_id;
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.replace_terminals(vec![current]);
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    handle(
        &mut state,
        &jobs,
        hub,
        key(KeyCode::Char('n'), KeyModifiers::NONE),
    );
    handle(
        &mut state,
        &jobs,
        hub,
        key(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    handle(
        &mut state,
        &jobs,
        hub,
        Event::Paste(format!("{DIRECTORY}\n")),
    );
    assert_eq!(state.entry, DIRECTORY);
    assert_eq!(state.mode, Mode::Directory);
    assert!(incoming.try_recv().is_err());
    handle(
        &mut state,
        &jobs,
        hub,
        key(KeyCode::Enter, KeyModifiers::NONE),
    );
    let Job::Call {
        command: Command::Create { request },
        ..
    } = incoming.try_recv().unwrap()
    else {
        panic!("directory submission must create a shell");
    };
    assert_eq!(request.fields().cwd, DIRECTORY);
    assert!(incoming.try_recv().is_err());
}

#[test]
fn pasted_filter_reconciles_results_and_reopening_starts_editing_at_the_end() {
    let current = fixtures::session();
    let hub = current.fields().address.hub_id;
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![current]);
    crate::tabs::terminal_picker::open(&mut state);
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    handle(
        &mut state,
        &jobs,
        hub,
        key(KeyCode::Char('/'), KeyModifiers::NONE),
    );
    handle(&mut state, &jobs, hub, Event::Paste(FILTER.into()));
    assert_eq!(state.filter, FILTER);
    assert!(state.visible().is_empty());
    handle(
        &mut state,
        &jobs,
        hub,
        key(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert_eq!(state.visible().len(), 1);
    handle(&mut state, &jobs, hub, Event::Paste("Term".into()));
    handle(
        &mut state,
        &jobs,
        hub,
        key(KeyCode::Home, KeyModifiers::NONE),
    );
    handle(
        &mut state,
        &jobs,
        hub,
        key(KeyCode::Esc, KeyModifiers::NONE),
    );
    handle(
        &mut state,
        &jobs,
        hub,
        key(KeyCode::Char('/'), KeyModifiers::NONE),
    );
    handle(&mut state, &jobs, hub, Event::Paste("inal".into()));
    assert_eq!(state.filter, fixtures::SESSION_NAME);
    assert_eq!(state.visible().len(), 1);
    assert!(incoming.try_recv().is_err());
}

#[test]
fn moving_within_a_directory_keeps_combining_characters_and_does_not_dispatch_to_pty() {
    let hub = yolo_hub_identity::HubId::new();
    let mut state = crate::client_capability_support::state();
    state.mode = Mode::Directory;
    state.entry = "/tmp/e\u{301}中".into();
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    handle(
        &mut state,
        &jobs,
        hub,
        key(KeyCode::Left, KeyModifiers::NONE),
    );
    handle(
        &mut state,
        &jobs,
        hub,
        key(KeyCode::Backspace, KeyModifiers::NONE),
    );
    handle(&mut state, &jobs, hub, Event::Paste("目录".into()));
    assert_eq!(state.entry, "/tmp/目录中");
    assert!(incoming.try_recv().is_err());
}
