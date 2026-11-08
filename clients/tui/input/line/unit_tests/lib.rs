mod editing;
mod paste_budget;
mod prefilled_controls;
mod viewport;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use yolo_tui_line_input::Cursor;

const LIMIT: usize = 128;

fn key(cursor: &mut Cursor, text: &mut String, code: KeyCode) {
    cursor.handle(
        text,
        &Event::Key(KeyEvent::new(code, KeyModifiers::NONE)),
        LIMIT,
    );
}

fn control(cursor: &mut Cursor, text: &mut String, code: char) {
    cursor.handle(
        text,
        &Event::Key(KeyEvent::new(KeyCode::Char(code), KeyModifiers::CONTROL)),
        LIMIT,
    );
}
