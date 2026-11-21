use super::*;
use crossterm::event::KeyEventKind;

#[test]
fn unicode_edits_move_and_delete_whole_visible_characters() {
    let mut cursor = Cursor::default();
    let mut text = "Ae\u{301}👩‍💻中".to_owned();
    key(&mut cursor, &mut text, KeyCode::Left);
    key(&mut cursor, &mut text, KeyCode::Backspace);
    assert_eq!(text, "Ae\u{301}中");
    key(&mut cursor, &mut text, KeyCode::Left);
    key(&mut cursor, &mut text, KeyCode::Delete);
    assert_eq!(text, "A中");
    cursor.handle(&mut text, &Event::Paste("目录".into()), LIMIT);
    assert_eq!(text, "A目录中");
    key(&mut cursor, &mut text, KeyCode::Right);
    key(&mut cursor, &mut text, KeyCode::Right);
    assert_eq!(cursor.position(&text), text.len());
}

#[test]
fn home_end_and_control_editing_replace_prefilled_values() {
    let mut cursor = Cursor::default();
    let mut text = "old directory".to_owned();
    control(&mut cursor, &mut text, 'u');
    assert!(text.is_empty());
    cursor.handle(&mut text, &Event::Paste("/tmp/file".into()), LIMIT);
    control(&mut cursor, &mut text, 'a');
    key(&mut cursor, &mut text, KeyCode::Right);
    control(&mut cursor, &mut text, 'k');
    assert_eq!(text, "/");
    control(&mut cursor, &mut text, 'e');
    cursor.handle(&mut text, &Event::Paste("目录".into()), LIMIT);
    key(&mut cursor, &mut text, KeyCode::Home);
    key(&mut cursor, &mut text, KeyCode::Backspace);
    assert_eq!(cursor.position(&text), 0);
    key(&mut cursor, &mut text, KeyCode::End);
    key(&mut cursor, &mut text, KeyCode::Delete);
    assert_eq!(text, "/目录");
}

#[test]
fn paste_strips_control_characters_and_bounds_complete_graphemes() {
    let mut cursor = Cursor::default();
    let mut text = String::new();
    cursor.handle(
        &mut text,
        &Event::Paste("A\n\r\t\0\x1b中e\u{301}".into()),
        "A中".len(),
    );
    assert_eq!(text, "A中");
    let full = text.len();
    assert!(!cursor.handle(&mut text, &Event::Paste("B".into()), full));
    let mut empty = String::new();
    cursor.reset();
    assert!(!cursor.handle(&mut empty, &Event::Paste("e\u{301}".into()), "e".len()));
    assert!(empty.is_empty());
}

#[test]
fn replacing_an_external_value_never_leaves_an_invalid_byte_cursor() {
    let mut cursor = Cursor::default();
    let mut text = "long value".to_owned();
    key(&mut cursor, &mut text, KeyCode::Left);
    text = "中".into();
    key(&mut cursor, &mut text, KeyCode::Backspace);
    assert!(text.is_empty());
    text = "ab".into();
    cursor.reset();
    key(&mut cursor, &mut text, KeyCode::Left);
    text = "👩‍💻Z".into();
    key(&mut cursor, &mut text, KeyCode::Delete);
    assert_eq!(text, "Z");
    cursor.reset();
    key(&mut cursor, &mut text, KeyCode::Char('!'));
    assert_eq!(text, "Z!");
}

#[test]
fn insertion_recomputes_a_combining_boundary_and_never_submits_pasted_enter() {
    let mut cursor = Cursor::default();
    let mut text = "eX".to_owned();
    key(&mut cursor, &mut text, KeyCode::Left);
    cursor.handle(&mut text, &Event::Paste("\u{301}\n".into()), LIMIT);
    key(&mut cursor, &mut text, KeyCode::Backspace);
    assert_eq!(text, "X");
}

#[test]
fn unrelated_events_and_modified_shortcuts_do_not_insert_text() {
    let mut cursor = Cursor::default();
    let mut text = "retained".to_owned();
    for event in [
        Event::FocusLost,
        Event::Resize(1, 1),
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('\n'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)),
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL)),
        Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        )),
    ] {
        assert!(!cursor.handle(&mut text, &event, LIMIT));
    }
    assert_eq!(text, "retained");
    cursor.handle(
        &mut text,
        &Event::Key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT)),
        LIMIT,
    );
    assert_eq!(text, "retainedX");
}
