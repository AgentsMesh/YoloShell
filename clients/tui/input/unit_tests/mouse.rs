use super::mouse_constants::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use yolo_tui_input::encode_mouse;

pub fn event(kind: MouseEventKind) -> MouseEvent {
    MouseEvent {
        kind,
        row: 0,
        column: 0,
        modifiers: KeyModifiers::NONE,
    }
}
pub fn parser(mode: &[u8], columns: u16) -> vt100::Parser {
    let mut parser = vt100::Parser::new(CELL, columns, 0);
    parser.process(mode);
    parser
}

#[test]
fn enabled_modes_emit_exact_sgr_press_release_drag_and_wheel_bytes() {
    use MouseEventKind::*;
    let screen = parser(SGR, TEST_COLUMNS);
    for (kind, expected) in [
        (Down(MouseButton::Left), SGR_LEFT),
        (Down(MouseButton::Middle), SGR_MIDDLE),
        (Down(MouseButton::Right), SGR_RIGHT),
        (Up(MouseButton::Left), SGR_RELEASE),
        (ScrollUp, WHEELS[0]),
        (ScrollDown, WHEELS[1]),
        (ScrollLeft, WHEELS[2]),
        (ScrollRight, WHEELS[3]),
    ] {
        assert_eq!(
            encode_mouse(event(kind), screen.screen()).unwrap(),
            expected
        );
    }
    assert!(
        encode_mouse(event(Drag(MouseButton::Left)), screen.screen())
            .unwrap()
            .is_empty()
    );
    assert!(
        encode_mouse(event(Moved), screen.screen())
            .unwrap()
            .is_empty()
    );
    let screen = parser(DRAG, TEST_COLUMNS);
    assert_eq!(
        encode_mouse(event(Drag(MouseButton::Left)), screen.screen()).unwrap(),
        SGR_DRAG
    );
    assert!(
        encode_mouse(event(Moved), screen.screen())
            .unwrap()
            .is_empty()
    );
    let screen = parser(MOTION, TEST_COLUMNS);
    assert_eq!(
        encode_mouse(event(Moved), screen.screen()).unwrap(),
        SGR_MOTION
    );
}

#[test]
fn modifiers_and_disabled_x10_release_rules_are_not_inferred_from_screen_text() {
    let mut pressed = event(MouseEventKind::Down(MouseButton::Left));
    pressed.modifiers = KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CONTROL;
    let screen = parser(SGR, TEST_COLUMNS);
    assert_eq!(
        encode_mouse(pressed, screen.screen()).unwrap(),
        SGR_MODIFIED
    );
    let screen = parser(X10, TEST_COLUMNS);
    assert_eq!(encode_mouse(pressed, screen.screen()).unwrap(), LEGACY_LEFT);
    assert!(
        encode_mouse(
            event(MouseEventKind::Up(MouseButton::Left)),
            screen.screen()
        )
        .unwrap()
        .is_empty()
    );
    let screen = parser(b"", TEST_COLUMNS);
    assert!(encode_mouse(pressed, screen.screen()).unwrap().is_empty());
    let screen = parser(DEFAULT, TEST_COLUMNS);
    assert_eq!(
        encode_mouse(
            event(MouseEventKind::Up(MouseButton::Left)),
            screen.screen()
        )
        .unwrap(),
        LEGACY_RELEASE
    );
}
