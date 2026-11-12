use crate::{State, network::Job};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;
use tokio::sync::mpsc;

pub(crate) fn click(state: &mut State, jobs: &mpsc::Sender<Job>, event: MouseEvent) -> bool {
    let position = Position::new(event.column, event.row);
    let Some(tab) = state
        .presentation
        .tab_strip
        .iter()
        .find(|tab| tab.area.contains(position))
    else {
        return false;
    };
    if !matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) || !event.modifiers.is_empty()
    {
        return true;
    }
    let current = state
        .tabs
        .entries()
        .iter()
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    if current != state.presentation.tab_ids
        || !state.presentation.current(&state.tab_layout(), state.mode)
    {
        return true;
    }
    let id = tab.id;
    if super::activate(state, id) {
        crate::polling::acquire(state, jobs, false);
    }
    true
}
