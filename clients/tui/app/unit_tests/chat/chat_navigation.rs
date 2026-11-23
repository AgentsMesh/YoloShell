use super::chat_support::*;
use crate::{Mode, chat};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};

#[test]
fn all_composer_navigation_stays_local_and_key_release_never_submits() {
    let (mut state, jobs, mut incoming) = ready();
    assert!(!chat::handle(&mut state, &jobs, &Event::FocusLost));
    toggle(&mut state, &jobs);
    assert!(!chat::handle(
        &mut state,
        &jobs,
        &Event::Resize(WIDTH, HEIGHT)
    ));
    let mut release = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    release.kind = KeyEventKind::Release;
    assert!(chat::handle(&mut state, &jobs, &Event::Key(release)));
    for code in [
        KeyCode::Char('a'),
        KeyCode::Tab,
        KeyCode::Char('b'),
        KeyCode::Home,
        KeyCode::Right,
        KeyCode::Delete,
        KeyCode::End,
        KeyCode::Backspace,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Up,
    ] {
        assert!(chat::handle(&mut state, &jobs, &Event::Key(code.into())));
    }
    for kind in [MouseEventKind::ScrollUp, MouseEventKind::ScrollDown] {
        assert!(chat::handle(
            &mut state,
            &jobs,
            &Event::Mouse(MouseEvent {
                kind,
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE
            })
        ));
    }
    assert!(incoming.try_recv().is_err());
    paste(&mut state, &jobs, "\u{1b}[201~");
    assert!(state.status.contains("control characters"));
    key(&mut state, &jobs, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert!(chat::current(&state).unwrap().composer.text().is_empty());
    state.chats.entries.clear();
    assert!(chat::handle(
        &mut state,
        &jobs,
        &Event::Paste(PROMPT.into())
    ));
}

#[test]
fn cache_capacity_evicts_empty_views_and_never_silently_discards_user_drafts() {
    let (mut state, jobs, mut incoming) = ready();
    let address = state.focused_terminal().unwrap();
    for index in 0..VIEW_LIMIT {
        state.mode = Mode::Terminal;
        state.tabs = state.tab_snapshot();
        state.tabs.create(&format!("Chat {index}")).unwrap();
        state
            .tabs
            .replace_active_layout(yolo_presentation_layout::Layout::new(Some(address)));
        let catalog = state.tab_snapshot();
        state.tabs = catalog;
        toggle(&mut state, &jobs);
        paste(&mut state, &jobs, PROMPT);
    }
    assert_eq!(state.chats.entries.len(), VIEW_LIMIT);
    state.mode = Mode::Terminal;
    state.tabs = state.tab_snapshot();
    state.tabs.create("Extra Chat").unwrap();
    state
        .tabs
        .replace_active_layout(yolo_presentation_layout::Layout::new(Some(address)));
    let catalog = state.tab_snapshot();
    state.tabs = catalog;
    toggle(&mut state, &jobs);
    assert_eq!(state.mode, Mode::Terminal);
    assert_eq!(state.chats.entries.len(), VIEW_LIMIT);
    assert!(state.status.contains("Chat drafts are retained"));
    let first = state.chats.entries.keys().next().copied().unwrap();
    state
        .chats
        .entries
        .get_mut(&first)
        .unwrap()
        .view
        .composer
        .clear()
        .unwrap();
    toggle(&mut state, &jobs);
    assert_eq!(state.mode, Mode::Chat);
    assert!(!state.chats.entries.contains_key(&first));
    assert_eq!(state.chats.entries.len(), VIEW_LIMIT);
    assert!(incoming.try_recv().is_err());
}
