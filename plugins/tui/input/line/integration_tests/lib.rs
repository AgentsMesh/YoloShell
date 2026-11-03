use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use yolo_tui_line_input::{Cursor, render};

mod prefilled_controls;

const TEXT: &str = "/very/long/path/with/中文/e\u{301}/tail";
const LABEL: &str = "Directory: ";
const HINT: &str = " · Enter confirms · Esc cancels";
const LIMIT: usize = 512;

#[test]
fn actual_terminal_cursor_tracks_the_visible_edit_position_at_every_width() {
    let mut cursor = Cursor::default();
    let mut text = TEXT.to_owned();
    for code in [KeyCode::End, KeyCode::Home, KeyCode::Right, KeyCode::End] {
        cursor.handle(
            &mut text,
            &Event::Key(KeyEvent::new(code, KeyModifiers::NONE)),
            LIMIT,
        );
        for columns in [1, 2, 8, 20, 80, 160] {
            let mut terminal = Terminal::new(TestBackend::new(columns, 2)).unwrap();
            terminal
                .draw(|frame| render(frame, frame.area(), LABEL, HINT, &text, &cursor))
                .unwrap();
            let position = terminal.get_cursor_position().unwrap();
            assert!(position.x < columns);
            assert_eq!(position.y, 0);
            let content = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            if columns >= 80 {
                assert!(content.contains(LABEL));
                assert!(content.contains(HINT));
            }
            if code == KeyCode::End && columns >= 8 {
                assert!(content.contains("tail"));
            }
        }
    }
}

#[test]
fn empty_render_area_and_zero_width_never_move_cursor_outside_the_terminal() {
    let mut terminal = Terminal::new(TestBackend::new(1, 1)).unwrap();
    terminal
        .draw(|frame| {
            render(
                frame,
                Rect::default(),
                LABEL,
                HINT,
                TEXT,
                &Cursor::default(),
            )
        })
        .unwrap();
    assert_eq!(terminal.backend().buffer().content().len(), 1);
}
