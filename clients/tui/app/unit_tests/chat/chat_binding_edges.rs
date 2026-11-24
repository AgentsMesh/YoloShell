use super::{chat_support::*, interrupt_support};
use crate::{Mode, chat};
use crossterm::event::{KeyCode, KeyModifiers};

#[test]
fn absent_or_inconsistent_terminal_metadata_cannot_open_a_chat_for_the_wrong_runtime() {
    for scenario in 0..4 {
        let (mut state, jobs, mut incoming) = ready();
        let address = state.focused_terminal().unwrap();
        match scenario {
            0 => {
                state.bind_focused(None).unwrap();
            }
            1 => {
                let mut fields = state.terminals[&address].fields().clone();
                fields.runtime = None;
                state.terminals.insert(
                    address,
                    yolo_hub_terminal_protocol::TerminalSummary::new(fields).unwrap(),
                );
            }
            2 => {
                state.inventory.agents.clear();
            }
            _ => interrupt_support::mutate_agent(&mut state, |fields| {
                fields.terminal.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new()
            }),
        }
        toggle(&mut state, &jobs);
        assert_eq!(state.mode, Mode::Terminal);
        assert!(state.chats.entries.is_empty());
        assert!(incoming.try_recv().is_err());
    }
}

#[test]
fn picker_close_restores_chat_while_its_entry_is_alive() {
    let (mut state, jobs, _incoming) = ready();
    toggle(&mut state, &jobs);
    assert_eq!(state.mode, Mode::Chat);
    assert!(chat::current(&state).is_some());
    crate::tabs::terminal_picker::open(&mut state);
    assert_eq!(state.mode, Mode::Inventory);
    crate::tabs::terminal_picker::close(&mut state);
    assert_eq!(state.mode, Mode::Chat);
    assert!(chat::current(&state).is_some());
}

#[test]
fn picker_close_does_not_restore_chat_without_an_entry() {
    let (mut state, _jobs, _incoming) = ready();
    state.mode = Mode::Chat;
    assert!(chat::current(&state).is_none());
    crate::tabs::terminal_picker::open(&mut state);
    crate::tabs::terminal_picker::close(&mut state);
    assert_eq!(state.mode, Mode::Terminal);
}

#[test]
fn closing_a_pane_without_a_chat_returns_to_terminal_input() {
    let (mut state, _jobs, _incoming) = ready();
    let address = state.focused_terminal().unwrap();
    let second = state
        .tab_layout_mut()
        .split(yolo_presentation_layout::Axis::Horizontal, Some(address))
        .unwrap();
    state.tab_layout_mut().focus(second).unwrap();
    state.mode = Mode::Chat;
    assert!(chat::current(&state).is_none());
    crate::tabs::close::pane(&mut state);
    assert_eq!(state.mode, Mode::Terminal);
}

#[test]
fn closing_one_pane_never_discards_another_panes_local_draft() {
    let (mut state, jobs, mut incoming) = ready();
    let address = state.focused_terminal().unwrap();
    let tab = state.tabs.active().id;
    let first = state.tab_layout().focused();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Esc, KeyModifiers::NONE);
    let second = state
        .tab_layout_mut()
        .split(yolo_presentation_layout::Axis::Horizontal, Some(address))
        .unwrap();
    state.tab_layout_mut().focus(second).unwrap();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, NEXT_PROMPT);
    chat::close(&mut state, tab, Some(first));
    assert_eq!(state.chats.entries.len(), 1);
    assert_eq!(chat::current(&state).unwrap().composer.text(), NEXT_PROMPT);
    chat::close(&mut state, uuid::Uuid::new_v4(), None);
    assert_eq!(state.chats.entries.len(), 1);
    assert!(incoming.try_recv().is_err());
}
