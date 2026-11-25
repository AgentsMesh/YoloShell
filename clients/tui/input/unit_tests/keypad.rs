use crate::constants::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventState, KeyModifiers};
use yolo_tui_input::{InputError, encode_key};

#[test]
fn application_keypad_encodes_only_distinguishable_keypad_events() {
    let mut parser = vt100::Parser::new(ROWS, COLUMNS, 0);
    parser.process(ENABLE_APPLICATION_KEYPAD);
    for character in "0123456789.,+-*/=".chars() {
        let ordinary = KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
        assert_eq!(
            encode_key(ordinary, parser.screen()).unwrap(),
            character.to_string().as_bytes()
        );
        let mut keypad = ordinary;
        keypad.state = KeyEventState::KEYPAD;
        assert!(
            encode_key(keypad, parser.screen())
                .unwrap()
                .starts_with(b"\x1bO")
        );
        keypad.modifiers = KeyModifiers::CONTROL;
        assert_eq!(
            encode_key(keypad, parser.screen()),
            Err(InputError::UnsupportedKey)
        );
    }
    let mut enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    enter.state = KeyEventState::KEYPAD;
    assert_eq!(encode_key(enter, parser.screen()).unwrap(), b"\x1bOM");
    enter.code = KeyCode::Null;
    assert_eq!(
        encode_key(enter, parser.screen()),
        Err(InputError::UnsupportedKey)
    );
}
