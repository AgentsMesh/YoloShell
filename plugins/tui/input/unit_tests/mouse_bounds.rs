use super::{
    mouse::{event, parser},
    mouse_constants::*,
};
use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};
use yolo_tui_input::{InputError, encode_mouse};

#[test]
fn legacy_utf8_and_sgr_coordinates_never_wrap_or_escape_the_source_terminal() {
    for (mode, limit, expected) in [
        (DEFAULT, LEGACY_LAST_COLUMN, LEGACY_EDGE),
        (UTF8, UTF8_LAST_COLUMN, UTF8_EDGE),
    ] {
        let screen = parser(mode, limit + CELL + CELL);
        let mut pressed = event(MouseEventKind::Down(MouseButton::Left));
        pressed.column = limit;
        assert_eq!(encode_mouse(pressed, screen.screen()).unwrap(), expected);
        pressed.column += CELL;
        assert_eq!(
            encode_mouse(pressed, screen.screen()),
            Err(InputError::MouseCoordinate)
        );
        pressed.column = 0;
        pressed.row = CELL;
        assert_eq!(
            encode_mouse(pressed, screen.screen()),
            Err(InputError::MouseCoordinate)
        );
    }
    let screen = parser(SGR, u16::MAX);
    let mut pressed = event(MouseEventKind::Down(MouseButton::Left));
    pressed.column = u16::MAX - CELL;
    assert_eq!(encode_mouse(pressed, screen.screen()).unwrap(), SGR_EDGE);
    pressed.column = u16::MAX;
    assert_eq!(
        encode_mouse(pressed, screen.screen()),
        Err(InputError::MouseCoordinate)
    );
}

#[test]
fn unsupported_modifiers_are_explicit_errors_and_mouse_errors_are_described() {
    let screen = parser(SGR, TEST_COLUMNS);
    for modifiers in [KeyModifiers::SUPER, KeyModifiers::HYPER, KeyModifiers::META] {
        let mut pressed = event(MouseEventKind::Down(MouseButton::Left));
        pressed.modifiers = modifiers;
        assert_eq!(
            encode_mouse(pressed, screen.screen()),
            Err(InputError::UnsupportedMouse)
        );
    }
    assert!(!InputError::UnsupportedMouse.to_string().is_empty());
    assert!(!InputError::MouseCoordinate.to_string().is_empty());
}
