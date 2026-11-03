use super::Viewport;
use crate::help_constants::*;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

pub(super) fn draw(frame: &mut Frame<'_>, requested: u16) -> Viewport {
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(HELP_FOOTER_ROWS)])
        .split(frame.area());
    let block = Block::default().borders(Borders::ALL).title(HELP_TITLE);
    let body = block.inner(rows[0]);
    let lines = yolo_tui_render::wrap(HELP_TEXT, body.width);
    let total = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    let maximum = total.saturating_sub(body.height);
    let offset = requested.min(maximum);
    frame.render_widget(block, rows[0]);
    frame.render_widget(
        Paragraph::new(
            lines
                .into_iter()
                .skip(usize::from(offset))
                .collect::<Vec<_>>(),
        ),
        body,
    );
    let hint = [HELP_FOOTER, HELP_COMPACT_FOOTER, HELP_CLOSE_HINT]
        .into_iter()
        .find(|hint| Line::raw(*hint).width() <= usize::from(rows[1].width))
        .unwrap_or(HELP_CLOSE_HINT);
    frame.render_widget(Paragraph::new(hint), rows[1]);
    Viewport {
        offset,
        rows: body.height,
        maximum,
    }
}
