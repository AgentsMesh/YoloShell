use crate::{State, constants::*};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};

/// Healthy inventory views show no redundant title line: the section tabs
/// already name the view and each row carries its own identity/status. Keep a
/// single line for the two states a user must not miss.
pub(crate) fn height(state: &State) -> u16 {
    u16::from(status(state).is_some())
}

pub(crate) fn draw(frame: &mut Frame<'_>, state: &State, area: Rect) {
    let Some((label, color)) = status(state) else {
        return;
    };
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(color)),
        area,
    );
}

fn status(state: &State) -> Option<(&'static str, Color)> {
    if state.control_generation.is_none() {
        Some((DISCONNECTED_LABEL, Color::Yellow))
    } else if !state.inventory.current {
        Some((CATALOG_REFRESHING_LABEL, Color::DarkGray))
    } else {
        None
    }
}
