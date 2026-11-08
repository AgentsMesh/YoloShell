use crate::{Cursor, graphemes::prefix};
use ratatui::{Frame, layout::Rect, text::Span, widgets::Paragraph};

const MIN_INPUT_COLUMNS: u16 = 8;

pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    label: &str,
    hint: &str,
    text: &str,
    cursor: &Cursor,
) {
    if area.is_empty() {
        return;
    }
    let label = prefix(
        label,
        usize::from(area.width.saturating_sub(MIN_INPUT_COLUMNS.min(area.width))),
    );
    let label_width = Span::raw(label).width() as u16;
    let remaining = area.width.saturating_sub(label_width);
    let hint_width = Span::raw(hint).width();
    let hint = if hint_width + usize::from(MIN_INPUT_COLUMNS) <= usize::from(remaining) {
        hint
    } else {
        ""
    };
    let hint_width = Span::raw(hint).width() as u16;
    let field_width = remaining.saturating_sub(hint_width);
    let viewport = cursor.viewport(text, field_width);
    frame.render_widget(
        Paragraph::new(label),
        Rect::new(area.x, area.y, label_width, area.height),
    );
    frame.render_widget(
        Paragraph::new(viewport.text),
        Rect::new(area.x + label_width, area.y, field_width, area.height),
    );
    frame.render_widget(
        Paragraph::new(hint),
        Rect::new(area.right() - hint_width, area.y, hint_width, area.height),
    );
    if let Some(column) = viewport.cursor_column {
        frame.set_cursor_position((area.x + label_width + column, area.y));
    }
}
