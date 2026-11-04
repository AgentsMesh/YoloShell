use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Marks a pane whose phase cannot accept input even though a retained screen
/// is still visible. The card is centered over the snapshot so it cannot be
/// mistaken for live terminal content.
pub(crate) fn render_action_overlay(frame: &mut Frame<'_>, area: Rect, label: &str, hint: &str) {
    if area.width < 12 || area.height < 3 {
        return;
    }
    let hint_columns = u16::try_from(hint.chars().count()).unwrap_or(u16::MAX);
    let width = area.width.min(hint_columns.saturating_add(4).max(24));
    let text_columns = width.saturating_sub(2).max(1);
    let lines = hint_columns
        .saturating_add(text_columns.saturating_sub(1))
        .checked_div(text_columns)
        .unwrap_or(1)
        .max(1);
    let height = lines.saturating_add(2).min(area.height);
    let rect = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {label} "))
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(hint)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        rect,
    );
}
