use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
};
use uuid::Uuid;

const STRIP_ROWS: u16 = 1;
const MAX_TAB_COLUMNS: u16 = 32;
const ACTIVE_COLOR: Color = Color::Cyan;
/// A fixed dark 256-color slot keeps the cyan label readable even when a
/// theme maps `DarkGray` to a relatively bright value.
const ACTIVE_BACKGROUND: Color = Color::Indexed(237);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TabStripItem {
    pub id: Uuid,
    pub label: String,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TabHit {
    pub id: Uuid,
    pub area: Rect,
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, items: &[TabStripItem]) -> Vec<TabHit> {
    if area.is_empty() || items.is_empty() {
        return Vec::new();
    }
    let labels = items
        .iter()
        .map(|item| Line::from(item.label.clone()))
        .collect::<Vec<_>>();
    let widths = labels
        .iter()
        .map(|label| {
            label
                .width()
                .min(usize::from(MAX_TAB_COLUMNS.min(area.width))) as u16
        })
        .collect::<Vec<_>>();
    let active = items.iter().position(|item| item.active).unwrap_or_default();
    let mut start = active;
    let mut used = widths[start];
    while let Some(previous) = start.checked_sub(1) {
        let proposed = used.saturating_add(widths[previous]);
        if proposed > area.width {
            break;
        }
        used = proposed;
        start = previous;
    }
    let mut tabs = Vec::new();
    let mut x = area.x;
    for (item, (label, width)) in items.iter().zip(labels.into_iter().zip(widths)).skip(start) {
        let width = width.min(area.right().saturating_sub(x));
        if width == 0 {
            break;
        }
        let tab_area = Rect::new(x, area.y, width, STRIP_ROWS);
        let style = if item.active {
            Style::default()
                .fg(ACTIVE_COLOR)
                .bg(ACTIVE_BACKGROUND)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        frame.render_widget(Paragraph::new(label).style(style), tab_area);
        tabs.push(TabHit {
            id: item.id,
            area: tab_area,
        });
        x = x.saturating_add(width);
    }
    tabs
}
