mod content;
mod wrap;
use crate::{Chat, InputReadiness, Mode, QueryError, constants::*};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
};

pub(crate) struct Row {
    text: String,
    color: Color,
}
pub(crate) struct Rendered {
    version: uuid::Uuid,
    width: u16,
    rows: Vec<Row>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Viewport {
    pub history: Rect,
    pub composer: Rect,
    pub total_rows: usize,
    pub first_row: usize,
}

/// Paints only the supplied rectangle. The caller owns pane layout and all PTY channels.
pub fn render(frame: &mut Frame<'_>, chat: &Chat, area: Rect, input: InputReadiness) -> Viewport {
    let area = area.intersection(frame.area());
    if area.is_empty() || chat.mode() != Mode::Chat {
        return Viewport::default();
    }
    let regions = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(COMPOSER_ROWS),
        Constraint::Length(STATUS_ROWS),
    ])
    .split(area);
    let mut cached = chat.rendered.borrow_mut();
    if cached.as_ref().is_none_or(|cache| {
        cache.version != chat.history.version || cache.width != regions[0].width
    }) {
        *cached = Some(Rendered {
            version: chat.history.version,
            width: regions[0].width,
            rows: content::rows(chat, regions[0].width),
        });
    }
    let rows = &cached.as_ref().expect("render cache was populated").rows;
    let bottom = rows.len().saturating_sub(usize::from(regions[0].height));
    let first_row = bottom.saturating_sub(chat.scroll);
    for (index, row) in rows
        .iter()
        .skip(first_row)
        .take(usize::from(regions[0].height))
        .enumerate()
    {
        frame.render_widget(
            Paragraph::new(row.text.as_str()).style(Style::default().fg(row.color)),
            Rect::new(
                regions[0].x,
                regions[0].y + index as u16,
                regions[0].width,
                CELL,
            ),
        );
    }
    composer(frame, chat, regions[1], input);
    frame.render_widget(
        Paragraph::new(status(chat, input)).style(Style::default().fg(Color::DarkGray)),
        regions[2],
    );
    Viewport {
        history: regions[0],
        composer: regions[1],
        total_rows: rows.len(),
        first_row,
    }
}

fn composer(frame: &mut Frame<'_>, chat: &Chat, mut area: Rect, input: InputReadiness) {
    if area.height == 0 {
        return;
    }
    frame.render_widget(
        Paragraph::new(if chat.retired() {
            RETIRED_HINT
        } else {
            match input {
                InputReadiness::Ready => APPEND_HINT,
                InputReadiness::AwaitingObservation => INPUT_REFRESHING,
                InputReadiness::Unavailable => INPUT_UNAVAILABLE,
            }
        })
        .style(Style::default().fg(Color::DarkGray)),
        Rect::new(area.x, area.y, area.width, CELL),
    );
    area.y += CELL;
    area.height = area.height.saturating_sub(CELL);
    let (rows, _) = wrap::wrap(chat.composer.text(), area.width, MAX_COMPOSER_ROWS);
    let (prefix, _) = wrap::wrap(
        &chat.composer.text()[..chat.composer.cursor()],
        area.width,
        MAX_COMPOSER_ROWS,
    );
    let row = prefix.len().saturating_sub(usize::from(CELL));
    let column = prefix
        .last()
        .map_or(0, |line| ratatui::text::Line::raw(line.as_str()).width());
    let wrapped = column == usize::from(area.width) && area.width != 0;
    let row = row + usize::from(wrapped);
    let column = if wrapped { 0 } else { column };
    let first = (row + usize::from(CELL)).saturating_sub(usize::from(area.height));
    for (index, text) in rows
        .iter()
        .skip(first)
        .take(usize::from(area.height))
        .enumerate()
    {
        frame.render_widget(
            Paragraph::new(text.as_str()),
            Rect::new(area.x, area.y + index as u16, area.width, CELL),
        );
    }
    if !chat.retired()
        && !chat.composer.locked()
        && row >= first
        && row - first < usize::from(area.height)
        && column < usize::from(area.width)
    {
        frame.set_cursor_position((area.x + column as u16, area.y + (row - first) as u16));
    }
}

fn status(chat: &Chat, input: InputReadiness) -> String {
    if chat.retired() {
        return RETIRED_HINT.into();
    }
    if let Some(phase) = chat.composer.phase() {
        return phase.label().into();
    }
    if let Some(error) = chat.error {
        return match error {
            QueryError::Conversation(error) => error.to_string(),
            QueryError::Unsupported => READER_UNSUPPORTED.into(),
            QueryError::Disconnected => READER_OFFLINE.into(),
            QueryError::InvalidResponse => INVALID_RESPONSE.into(),
        };
    }
    // A conditional watch keeps a query pending for the life of the view.
    // Only the first page is genuinely "loading"; once history is visible the
    // footer must describe the composer's readiness instead of flickering back
    // to a loading label on every watch cycle.
    if chat.pending.is_some() && chat.history.latest.is_none() {
        return LOADING.into();
    }
    if chat
        .history
        .latest
        .as_ref()
        .is_some_and(|page| !page.partial.is_empty())
    {
        return PARTIAL_HISTORY.into();
    }
    if chat.page_count() >= MAX_HISTORY_PAGES {
        return HISTORY_WINDOW.into();
    }
    match input {
        InputReadiness::Ready => READY,
        InputReadiness::AwaitingObservation => INPUT_REFRESHING,
        InputReadiness::Unavailable => INPUT_UNAVAILABLE,
    }
    .into()
}
