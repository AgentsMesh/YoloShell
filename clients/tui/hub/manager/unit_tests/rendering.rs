use super::fixture::*;
use crate::{
    fields::FieldId,
    model::{Action, Page},
    rows,
};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn key_and_identity_remain_readable_in_desktop_and_compact_terminal_sizes() {
    for (width, height) in [(120, 32), (80, 24), (40, 18), (24, 16), (1, 3), (0, 0)] {
        let manager = manager();
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| {
                let view = crate::render(frame, &manager, frame.area());
                assert!(
                    view.hits
                        .iter()
                        .all(|(rect, _)| frame.area().contains((rect.x, rect.y).into()))
                );
            })
            .unwrap();
        if width >= 40 {
            let text = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .replace(' ', "");
            assert!(text.contains(&manager.entry.to_string()));
            assert!(text.contains(&crate::wire_fields::key_text(
                manager.base.as_ref().unwrap().state.identity
            )));
        }
    }
}

#[test]
fn wrapped_fields_scroll_to_the_edit_cursor_and_pointer_events_are_generation_fenced() {
    let mut manager = manager();
    manager.page(Page::Peers);
    activate(&mut manager, Action::AddPeer);
    field(&mut manager, FieldId::Ssh, &"x".repeat(100));
    let mut terminal = Terminal::new(TestBackend::new(40, 16)).unwrap();
    let mut view = None;
    terminal
        .draw(|frame| view = Some(crate::render(frame, &manager, frame.area())))
        .unwrap();
    let view = view.unwrap();
    assert!(
        view.hits
            .iter()
            .any(|(_, index)| *index == manager.form.as_ref().unwrap().focused)
    );
    let (rect, _) = view.hits[0];
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x,
        row: rect.y,
        modifiers: KeyModifiers::NONE,
    };
    key(&mut manager, KeyCode::Esc);
    let before = manager.selected;
    assert!(manager.pointer(&view, click).is_none());
    assert_eq!(manager.selected, before);
}

#[test]
fn pointer_can_navigate_menu_and_confirmation_without_touching_terminal_processes() {
    let mut manager = manager();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut view = None;
    terminal
        .draw(|frame| view = Some(crate::render(frame, &manager, frame.area())))
        .unwrap();
    let view = view.unwrap();
    let (rect, _) = view.hits[0];
    manager.pointer(
        &view,
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.x,
            row: rect.y,
            modifiers: KeyModifiers::NONE,
        },
    );
    assert_eq!(manager.page, Page::Peers);
    assert!(manager.pending.is_none());
    assert_eq!(rows::wrap("a\u{754c}b", 3), vec!["a\u{754c}", "b"]);
    assert!(rows::wrap("text", 0).is_empty());
}

#[test]
fn the_header_reports_the_derived_peer_socket() {
    let mut manager = manager();
    let base = manager.base.as_mut().unwrap();
    base.peer_socket = Some("/run/user/1000/yolo/hub/peer.sock".into());
    manager.draft = Some(base.state.configuration.clone());
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| {
            crate::render(frame, &manager, frame.area());
        })
        .unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(text.contains("Peer socket: /run/user/1000/yolo/hub/peer.sock"));
}
