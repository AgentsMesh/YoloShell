use crate::{View, constants::*};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Paragraph, Tabs},
};

pub(crate) fn render(frame: &mut Frame<'_>, view: View, area: Rect) -> Vec<(Rect, View)> {
    let selected = match view {
        View::Terminals => {
            frame.render_widget(Paragraph::new(TERMINAL_PICKER_LABEL), area);
            return Vec::new();
        }
        View::Agents => AGENTS_SECTION,
        View::Machines => MACHINES_SECTION,
    };
    if area.width < COMPACT_SECTIONS_WIDTH {
        frame.render_widget(
            Paragraph::new(COMPACT_SECTION_LABELS[selected]).style(Style::default().fg(SELECTED_COLOR)),
            area,
        );
        return (!area.is_empty())
            .then(|| {
                (
                    Rect::new(
                        area.x,
                        area.y,
                        area.width.min(COMPACT_SECTION_LABELS[selected].len() as u16),
                        SECTION_ROWS,
                    ),
                    view,
                )
            })
            .into_iter()
            .collect();
    }
    let labels = if area.width < FULL_SECTIONS_WIDTH {
        COMPACT_SECTION_LABELS
    } else {
        SECTION_LABELS
    };
    frame.render_widget(
        Tabs::new(labels.map(Line::from))
            .select(selected)
            .divider(SECTION_DIVIDER)
            .padding(SECTION_PADDING, SECTION_PADDING)
            .highlight_style(Style::default().fg(SELECTED_COLOR)),
        area,
    );
    let mut x = area.x;
    let mut hits = Vec::new();
    for (label, view) in labels.into_iter().zip([View::Agents, View::Machines]) {
        x = x.saturating_add(SECTION_PADDING.len() as u16);
        let width = (label.len() as u16).min(area.right().saturating_sub(x));
        if width > 0 && area.height > 0 {
            hits.push((Rect::new(x, area.y, width, SECTION_ROWS), view));
        }
        x = x
            .saturating_add(width)
            .saturating_add((SECTION_PADDING.len() + SECTION_DIVIDER.len()) as u16);
    }
    hits
}
