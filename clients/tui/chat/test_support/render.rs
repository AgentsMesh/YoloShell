use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use yolo_tui_chat::{Chat, Viewport};

pub fn render(chat: &Chat, width: u16, height: u16) -> (String, Viewport) {
    render_with_input(chat, width, height, yolo_tui_chat::InputReadiness::Ready)
}

pub fn render_with_input(
    chat: &Chat,
    width: u16,
    height: u16,
    input: yolo_tui_chat::InputReadiness,
) -> (String, Viewport) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let mut viewport = Viewport::default();
    terminal
        .draw(|frame| {
            viewport = yolo_tui_chat::render(frame, chat, Rect::new(0, 0, width, height), input)
        })
        .unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content()
        .chunks(usize::from(width.max(1)))
        .map(row_text)
        .collect::<Vec<_>>()
        .join("\n");
    (text, viewport)
}

fn row_text(row: &[ratatui::buffer::Cell]) -> String {
    let mut text = String::new();
    let mut index = 0;
    while let Some(cell) = row.get(index) {
        text.push_str(cell.symbol());
        index += ratatui::text::Line::raw(cell.symbol()).width().max(1);
    }
    text
}
