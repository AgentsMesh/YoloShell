use crate::{
    Mode, State,
    constants::{DIRECTORY_LABEL, MAX_FILTER_BYTES, SEARCH_LABEL},
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};
use yolo_hub_terminal_protocol::MAX_DIRECTORY_BYTES;

const DIRECTORY_HINT: &str = " · Enter creates shell · Esc cancels";

#[derive(Default)]
pub(crate) struct FormState {
    cursor: yolo_tui_line_input::Cursor,
    error: Option<String>,
}

pub(crate) fn reset(state: &mut State) {
    state.form_input = FormState::default();
}

pub(crate) fn fail(state: &mut State, message: String) {
    state.form_input.error = Some(message);
}

pub(crate) fn rows(state: &State, width: u16) -> u16 {
    let error = matches!(state.mode, Mode::Search | Mode::Directory)
        .then_some(state.form_input.error.as_deref())
        .flatten();
    let extra = error.map_or(0, |message| yolo_tui_render::wrap(message, width).len());
    crate::constants::STATUS_ROWS.saturating_add(u16::try_from(extra).unwrap_or(u16::MAX))
}

/// Form editing owns all text payloads, including paste, before PTY dispatch.
/// Submission and cancellation remain with the form's lifecycle owner.
pub(crate) fn edit(state: &mut State, event: &Event) -> bool {
    if !matches!(state.mode, Mode::Search | Mode::Directory) {
        return false;
    }
    match event {
        Event::Key(key)
            if key.kind == KeyEventKind::Press
                && key.modifiers.difference(KeyModifiers::SHIFT).is_empty()
                && matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::F(1)) =>
        {
            if matches!(key.code, KeyCode::Enter | KeyCode::Esc) {
                state.form_input.error = None;
            }
            return false;
        }
        Event::Key(_) | Event::Paste(_) => {}
        _ => return false,
    }
    let changed = if state.mode == Mode::Search {
        state
            .form_input
            .cursor
            .handle(&mut state.filter, event, MAX_FILTER_BYTES)
    } else {
        state
            .form_input
            .cursor
            .handle(&mut state.entry, event, MAX_DIRECTORY_BYTES)
    };
    if changed && state.mode == Mode::Search {
        state.reconcile_selection();
    }
    if changed {
        state.form_input.error = None;
    }
    true
}

pub(crate) fn draw(frame: &mut Frame<'_>, state: &State, area: Rect) -> bool {
    let (label, text, hint) = match state.mode {
        Mode::Search => (SEARCH_LABEL, state.filter.as_str(), ""),
        Mode::Directory => (DIRECTORY_LABEL, state.entry.as_str(), DIRECTORY_HINT),
        _ => return false,
    };
    let mut hint = hint.to_owned();
    crate::status_bar::append_retained(state, &mut hint);
    yolo_tui_line_input::render(
        frame,
        Rect::new(
            area.x,
            area.y,
            area.width,
            area.height.min(crate::constants::STATUS_ROWS),
        ),
        &format!("{label}: "),
        &hint,
        text,
        &state.form_input.cursor,
    );
    if let Some(error) = &state.form_input.error {
        let error_area = Rect::new(
            area.x,
            area.y.saturating_add(crate::constants::STATUS_ROWS),
            area.width,
            area.height.saturating_sub(crate::constants::STATUS_ROWS),
        );
        frame.render_widget(
            Paragraph::new(yolo_tui_render::wrap(error, area.width))
                .style(Style::default().fg(Color::Yellow)),
            error_area,
        );
    }
    true
}
