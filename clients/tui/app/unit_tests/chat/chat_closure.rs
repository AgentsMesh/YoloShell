use super::chat_support::*;
use crate::{Mode, chat};
use crossterm::event::{KeyCode, KeyModifiers};

#[test]
fn explicit_view_close_discards_its_draft_without_terminating_the_bound_terminal() {
    let (mut state, jobs, mut incoming) = ready();
    let address = state.focused_terminal().unwrap();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Char('b'), KeyModifiers::CONTROL);
    key(&mut state, &jobs, KeyCode::Char('x'), KeyModifiers::NONE);
    assert!(state.chats.entries.is_empty());
    assert!(state.terminals.contains_key(&address));
    assert!(incoming.try_recv().is_err());
}

#[test]
fn remote_closed_view_draft_is_recovered_when_the_same_terminal_is_opened_again() {
    let (mut state, jobs, mut incoming) = ready();
    let address = state.focused_terminal().unwrap();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    let previous = state.tab_layout().focused();
    state.mode = Mode::Terminal;
    state
        .tabs
        .replace_active_layout(yolo_presentation_layout::Layout::new(Some(address)));
    assert_ne!(state.tab_layout().focused(), previous);
    toggle(&mut state, &jobs);
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    assert_eq!(state.chats.entries.len(), 1);
    assert!(incoming.try_recv().is_err());
}
