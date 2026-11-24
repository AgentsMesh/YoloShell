use crate::constants::STEP;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};
use uuid::Uuid;

pub struct View {
    pub(crate) epoch: Uuid,
    pub(crate) hits: Vec<(Rect, usize)>,
}
pub(crate) struct Row {
    pub text: String,
    pub style: Style,
    pub cursor: Option<usize>,
}
pub(crate) fn wrap(text: &str, width: u16) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    for source in text.split('\n') {
        let mut line = String::new();
        let mut used = 0;
        let span = Span::raw(source);
        for grapheme in span.styled_graphemes(Style::default()) {
            let size = Line::raw(grapheme.symbol).width();
            if used + size > usize::from(width) && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                used = 0;
            }
            if size <= usize::from(width) {
                line.push_str(grapheme.symbol);
                used += size;
            }
        }
        lines.push(line);
    }
    lines
}
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    items: Vec<Row>,
    selected: usize,
    epoch: Uuid,
) -> View {
    let mut physical = Vec::new();
    let mut focus_end = 0;
    let mut cursor = None;
    for (index, item) in items.into_iter().enumerate() {
        let offset = physical.len();
        let lines = wrap(&item.text, area.width);
        if index == selected {
            focus_end = offset + lines.len();
            if let Some(byte) = item.cursor {
                let prefix = wrap(&item.text[..byte], area.width);
                if let Some(last) = prefix.last() {
                    let width = Line::raw(last.as_str()).width();
                    let next_row = usize::from(area.width != 0 && width == usize::from(area.width));
                    cursor = Some((
                        offset + prefix.len().saturating_sub(STEP) + next_row,
                        if next_row == 0 { width as u16 } else { 0 },
                    ));
                }
            }
        }
        for text in lines {
            physical.push((index, text, item.style));
        }
    }
    let target = cursor.map_or(focus_end, |(row, _)| row + STEP);
    let scroll = target.saturating_sub(usize::from(area.height));
    let mut view = View {
        epoch,
        hits: Vec::new(),
    };
    for (row, (index, text, style)) in physical
        .into_iter()
        .skip(scroll)
        .take(usize::from(area.height))
        .enumerate()
    {
        let bounds = Rect::new(area.x, area.y + row as u16, area.width, 1);
        frame.render_widget(Paragraph::new(text).style(style), bounds);
        view.hits.push((bounds, index));
    }
    if let Some((row, column)) = cursor {
        if row >= scroll && row - scroll < usize::from(area.height) && column < area.width {
            frame.set_cursor_position((area.x + column, area.y + (row - scroll) as u16));
        }
    }
    view
}
