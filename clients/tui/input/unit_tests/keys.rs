use crate::constants::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use yolo_tui_input::{InputError, encode_key, encode_key_default};

#[test]
fn arrows_follow_remote_modes_and_xterm_modifier_encoding() {
    let mut parser = vt100::Parser::new(ROWS, COLUMNS, 0);
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(encode_key(key, parser.screen()).unwrap(), UP);
    parser.process(ENABLE_APPLICATION_CURSOR);
    assert_eq!(encode_key(key, parser.screen()).unwrap(), APPLICATION_UP);
    assert_eq!(
        encode_key(
            KeyEvent::new(
                KeyCode::Up,
                KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT
            ),
            parser.screen()
        )
        .unwrap(),
        MODIFIED_UP
    );
    for code in [
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Home,
        KeyCode::End,
    ] {
        assert!(encode_key(KeyEvent::new(code, KeyModifiers::NONE), parser.screen()).is_ok());
    }
}

#[test]
fn default_encoding_supports_keys_before_the_first_snapshot() {
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(encode_key_default(key).unwrap(), UP);
    assert_eq!(
        encode_key_default(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)).unwrap(),
        b"a"
    );
}

#[test]
fn characters_controls_release_and_unsupported_keys_are_distinct() {
    let parser = vt100::Parser::new(ROWS, COLUMNS, 0);
    for (code, modifiers, expected) in [
        (KeyCode::Char('a'), KeyModifiers::NONE, b"a".to_vec()),
        (KeyCode::Char('a'), KeyModifiers::ALT, b"\x1ba".to_vec()),
        (KeyCode::Char('c'), KeyModifiers::CONTROL, vec![3]),
        (KeyCode::Char('?'), KeyModifiers::CONTROL, vec![127]),
        (KeyCode::Char(' '), KeyModifiers::CONTROL, vec![0]),
        (KeyCode::Enter, KeyModifiers::NONE, b"\r".to_vec()),
        (KeyCode::Backspace, KeyModifiers::NONE, vec![127]),
        (KeyCode::Tab, KeyModifiers::NONE, b"\t".to_vec()),
        (KeyCode::Esc, KeyModifiers::NONE, ESCAPE.to_vec()),
    ] {
        assert_eq!(
            encode_key(KeyEvent::new(code, modifiers), parser.screen()).unwrap(),
            expected
        );
    }
    let mut released = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    released.kind = KeyEventKind::Release;
    assert_eq!(
        encode_key(released, parser.screen()).unwrap(),
        Vec::<u8>::new()
    );
    for key in [
        KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('2'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::SUPER),
    ] {
        assert_eq!(
            encode_key(key, parser.screen()),
            Err(InputError::UnsupportedKey)
        );
    }
}

#[test]
fn editing_and_function_keys_have_stable_encodings() {
    let parser = vt100::Parser::new(ROWS, COLUMNS, 0);
    for code in [
        KeyCode::Insert,
        KeyCode::Delete,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::BackTab,
    ] {
        for modifiers in [KeyModifiers::NONE, KeyModifiers::CONTROL] {
            assert!(encode_key(KeyEvent::new(code, modifiers), parser.screen()).is_ok());
        }
    }
    for number in 1..=MAX_FUNCTION_KEY {
        for modifiers in [KeyModifiers::NONE, KeyModifiers::ALT] {
            assert!(
                encode_key(
                    KeyEvent::new(KeyCode::F(number), modifiers),
                    parser.screen()
                )
                .is_ok()
            );
        }
    }
    for number in [0, MAX_FUNCTION_KEY + 1, u8::MAX] {
        assert_eq!(
            encode_key(
                KeyEvent::new(KeyCode::F(number), KeyModifiers::NONE),
                parser.screen()
            ),
            Err(InputError::UnsupportedKey)
        );
    }
}
