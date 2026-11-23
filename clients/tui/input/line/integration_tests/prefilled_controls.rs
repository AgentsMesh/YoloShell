use ratatui::{Terminal, backend::TestBackend, text::Span};
use yolo_tui_line_input::{Cursor, render};

const DIRECTORY: &str = "/tmp/\n中";
const LABEL: &str = "Directory: ";
const DISPLAY: &str = "/tmp/\\n中";
const WIDTH: u16 = 80;
const HEIGHT: u16 = 2;

#[test]
fn an_existing_newline_path_is_one_visible_row_without_changing_the_model() {
    let text = DIRECTORY.to_owned();
    let cursor = Cursor::default();
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| render(frame, frame.area(), LABEL, "", &text, &cursor))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let first = buffer.content()[..usize::from(WIDTH)]
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(first.contains("\\n"));
    assert!(first.contains('中'));
    assert!(
        buffer.content()[usize::from(WIDTH)..]
            .iter()
            .all(|cell| cell.symbol() == " ")
    );
    let position = terminal.get_cursor_position().unwrap();
    assert_eq!(position.y, 0);
    assert_eq!(
        usize::from(position.x),
        Span::raw(LABEL).width() + Span::raw(DISPLAY).width()
    );
    assert_eq!(text, DIRECTORY);
}
