use crate::{State, help_constants::*};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::Frame;

#[path = "help_render.rs"]
mod render;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Viewport {
    pub offset: u16,
    pub rows: u16,
    pub maximum: u16,
}

/// Application shortcuts are dispatched before this modal's local navigation.
pub(crate) fn handle(state: &mut State, event: &Event) -> bool {
    if !state.help {
        return false;
    }
    let viewport = state.presentation.help.unwrap_or_default();
    let offset = viewport.offset;
    let page = viewport.rows.max(HELP_SCROLL_STEP);
    let code = match event {
        Event::Mouse(mouse) => {
            state.help_offset = match mouse.kind {
                MouseEventKind::ScrollDown => offset.saturating_add(HELP_SCROLL_STEP),
                MouseEventKind::ScrollUp => offset.saturating_sub(HELP_SCROLL_STEP),
                _ => offset,
            }
            .min(viewport.maximum);
            return true;
        }
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            if !key.modifiers.difference(KeyModifiers::SHIFT).is_empty() {
                return true;
            }
            key.code
        }
        _ => return true,
    };
    match code {
        KeyCode::Esc | KeyCode::Char(HELP_KEY | QUIT_KEY) | KeyCode::F(1) => {
            state.help = false;
            state.help_offset = 0;
            state.prefix = false;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.help_offset = offset
                .saturating_add(HELP_SCROLL_STEP)
                .min(viewport.maximum);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.help_offset = offset.saturating_sub(HELP_SCROLL_STEP)
        }
        KeyCode::PageDown | KeyCode::Char(' ') => {
            state.help_offset = offset.saturating_add(page).min(viewport.maximum);
        }
        KeyCode::PageUp => state.help_offset = offset.saturating_sub(page),
        KeyCode::Home => state.help_offset = 0,
        KeyCode::End => state.help_offset = viewport.maximum,
        _ => {}
    }
    true
}

pub(crate) fn open(state: &mut State) {
    state.help = true;
    state.help_offset = 0;
    state.prefix = false;
    state.presentation = Default::default();
    crate::selection_actions::cancel(state);
    state.sync_query_authority();
}

pub(crate) fn draw(frame: &mut Frame<'_>, offset: u16) -> Viewport {
    render::draw(frame, offset)
}
