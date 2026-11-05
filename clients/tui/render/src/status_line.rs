//! Status-line layout: reserve state/action columns and elide a long title
//! without cutting UTF-8 or display cells.



/// Truncate a display string to at most `columns` terminal cells, keeping an
/// ellipsis. Character widths come from ratatui so CJK text is not cut by
/// bytes. Combining sequences can still split in the pathological case; the
/// terminal itself composes them normally.
pub fn elide(value: &str, columns: usize) -> String {
    if columns == 0 {
        return String::new();
    }
    if Line::raw(value).width() <= columns {
        return value.to_owned();
    }
    let mut result = String::new();
    let mut used = 0usize;
    for character in value.chars() {
        let width = Line::raw(character.to_string()).width();
        if used.saturating_add(width).saturating_add(1) > columns {
            break;
        }
        result.push(character);
        used = used.saturating_add(width);
    }
    result.push('…');
    result
}

use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

const MIN_TITLE_COLUMNS: usize = 16;

/// Reserve space for state and actions before a potentially long terminal title.
/// Paragraph clips whole display graphemes, so width is never a UTF-8 byte cut.
pub fn draw(frame: &mut Frame<'_>, line: Line<'_>, area: Rect, input_action: bool) {
    let Some((title, details)) = line.spans.split_first() else {
        return;
    };
    let details = Line::from(details.to_vec());
    let detail_columns = details.width().min(usize::from(area.width)) as u16;
    let title_floor = if input_action {
        0
    } else {
        MIN_TITLE_COLUMNS.min(usize::from(area.width))
    };
    let available = usize::from(area.width - detail_columns).max(title_floor);
    let title_columns = title.width().min(available) as u16;
    let title_area = Rect {
        width: title_columns,
        ..area
    };
    let detail_area = Rect {
        x: area.x.saturating_add(title_columns),
        width: area.width.saturating_sub(title_columns),
        ..area
    };
    let title = if title.width() > usize::from(title_columns) {
        Span::styled(
            elide(&title.content, usize::from(title_columns)),
            title.style,
        )
    } else {
        title.clone()
    };
    frame.render_widget(Paragraph::new(Line::from(title)), title_area);
    frame.render_widget(Paragraph::new(details), detail_area);
}
