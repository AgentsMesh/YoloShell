use super::{chat_support::*, interrupt_support};
use crate::{Mode, chat, network::Job};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};

#[test]
fn repeat_control_and_noneditor_mouse_events_never_become_terminal_bytes_or_duplicate_submission() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    let mut repeated = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    repeated.kind = KeyEventKind::Repeat;
    assert!(chat::handle(&mut state, &jobs, &Event::Key(repeated)));
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
        key(&mut state, &jobs, KeyCode::Char('a'), modifiers);
    }
    let mouse = MouseEvent {
        kind: MouseEventKind::Moved,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    assert!(!chat::handle(&mut state, &jobs, &Event::Mouse(mouse)));
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    assert!(incoming.try_recv().is_err());
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let _ = sent(&mut incoming);
    key(&mut state, &jobs, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert!(state.status.contains("previous submission"));
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    let id = chat::Key::current(&state);
    state.chats.entries.get_mut(&id).unwrap().view.retire();
    key(&mut state, &jobs, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    let mouse = MouseEvent {
        kind: MouseEventKind::ScrollUp,
        ..mouse
    };
    assert!(chat::handle(&mut state, &jobs, &Event::Mouse(mouse)));
}

#[test]
fn ordinary_x_is_composer_text_and_never_the_discard_shortcut() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Char('x'), KeyModifiers::NONE);
    assert_eq!(
        chat::current(&state).unwrap().composer.text(),
        format!("{PROMPT}x")
    );
    assert!(incoming.try_recv().is_err());
}

#[test]
fn reusing_an_empty_pane_for_a_new_execution_replaces_only_the_private_view() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    let previous = chat::current(&state).unwrap().binding();
    key(&mut state, &jobs, KeyCode::Esc, KeyModifiers::NONE);
    interrupt_support::mutate_agent(&mut state, |fields| {
        let mut target = fields.cursor.target();
        target.execution_id = yolo_hub_identity::ExecutionId::new();
        fields.cursor = yolo_hub_agent_protocol::ObservationCursor::new(
            target,
            uuid::Uuid::new_v4(),
            yolo_hub_agent_protocol::FIRST_OBSERVATION_SEQUENCE,
        )
        .unwrap();
    });
    toggle(&mut state, &jobs);
    assert_ne!(chat::current(&state).unwrap().binding(), previous);
    assert!(!chat::current(&state).unwrap().retired());
    assert_eq!(state.mode, Mode::Chat);
    assert!(incoming.try_recv().is_err());
    state.mode = Mode::Terminal;
    chat::pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
}

#[test]
fn submit_revalidates_mode_and_runtime_after_a_ui_intent_was_created() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    state.mode = Mode::Inventory;
    chat::submit_for_test(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    state.mode = Mode::Chat;
    let address = state.focused_terminal().unwrap();
    let mut fields = *state.writers[&address].grant.fields();
    fields.target.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
    state.writers.get_mut(&address).unwrap().grant =
        yolo_hub_terminal_protocol::LeaseGrant::new(fields).unwrap();
    chat::submit_for_test(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    let mut fields = fields;
    fields.target = state.terminals[&address].fields().runtime.unwrap().target;
    state.writers.get_mut(&address).unwrap().grant =
        yolo_hub_terminal_protocol::LeaseGrant::new(fields).unwrap();
    state.chats.entries.clear();
    chat::submit_for_test(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
}
