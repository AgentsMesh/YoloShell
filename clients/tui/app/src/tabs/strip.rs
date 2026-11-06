use crate::State;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
};
use uuid::Uuid;
use yolo_tui_tab::TabEntry;

const STRIP_ROWS: u16 = 1;
const MAX_TAB_COLUMNS: u16 = 32;
const FIRST_TAB_NUMBER: usize = 1;
const ACTIVE_MARKER: &str = "●";
const INACTIVE_MARKER: &str = " ";
const TAB_PADDING: &str = " ";
const ACTIVE_COLOR: Color = Color::Cyan;
/// A fixed dark 256-color slot keeps the cyan label readable even when a
/// theme maps `DarkGray` to a relatively bright value.
const ACTIVE_BACKGROUND: Color = Color::Indexed(237);
const ENDED_TAB_MARKER: &str = "×";
const LOST_TAB_MARKER: &str = "!";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Tab {
    pub id: Uuid,
    pub area: Rect,
}

pub(crate) fn draw(frame: &mut Frame<'_>, state: &State, area: Rect) -> Vec<Tab> {
    if area.is_empty() {
        return Vec::new();
    }
    let entries = state.tabs.entries();
    let active = state.tabs.active().id;
    let labels = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let marker = if entry.id == active {
                ACTIVE_MARKER
            } else {
                INACTIVE_MARKER
            };
            let number = index + FIRST_TAB_NUMBER;
            let title = title(state, entry);
            Line::from(format!(
                "{TAB_PADDING}{marker}{number} {title}{TAB_PADDING}"
            ))
        })
        .collect::<Vec<_>>();
    let widths = labels
        .iter()
        .map(|label| {
            label
                .width()
                .min(usize::from(MAX_TAB_COLUMNS.min(area.width))) as u16
        })
        .collect::<Vec<_>>();
    let mut start = entries
        .iter()
        .position(|entry| entry.id == active)
        .unwrap_or_default();
    let mut used = widths[start];
    while let Some(previous) = start.checked_sub(FIRST_TAB_NUMBER) {
        let proposed = used.saturating_add(widths[previous]);
        if proposed > area.width {
            break;
        }
        used = proposed;
        start = previous;
    }
    let mut tabs = Vec::new();
    let mut x = area.x;
    for ((entry, label), width) in entries.iter().zip(labels).zip(widths).skip(start) {
        let width = width.min(area.right().saturating_sub(x));
        if width == 0 {
            break;
        }
        let tab_area = Rect::new(x, area.y, width, STRIP_ROWS);
        let style = if entry.id == active {
            Style::default()
                .fg(ACTIVE_COLOR)
                .bg(ACTIVE_BACKGROUND)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        frame.render_widget(Paragraph::new(label).style(style), tab_area);
        tabs.push(Tab {
            id: entry.id,
            area: tab_area,
        });
        x = x.saturating_add(width);
    }
    tabs
}

fn title(state: &State, entry: &TabEntry) -> String {
    let layout = if entry.id == state.tabs.active().id {
        &state.tab_layout()
    } else {
        &entry.view.layout
    };
    // Tabs are named navigation resources. Dynamic terminal titles belong to
    // the pane and status line; they must not replace a user's tab name.
    let title = entry.name.to_owned();
    let address = layout
        .panes()
        .into_iter()
        .find(|pane| pane.id == layout.focused())
        .and_then(|pane| pane.terminal);
    // A tab whose terminal can never accept input must not look like a live
    // one in the strip. The phase label remains in the pane and status bar.
    let phase_marker =
        address.and_then(
            |address| match crate::terminal_lifecycle::phase(state, address) {
                yolo_tui_lifecycle::TerminalPhase::Ended
                | yolo_tui_lifecycle::TerminalPhase::Archived => Some(ENDED_TAB_MARKER),
                yolo_tui_lifecycle::TerminalPhase::Lost => Some(LOST_TAB_MARKER),
                _ => None,
            },
        );
    if let Some(marker) = phase_marker {
        return format!("{marker} {title}");
    }
    let control_marker = address.and_then(|address| crate::control::marker(state, address));
    control_marker.map_or(title.clone(), |marker| format!("{marker} {title}"))
}
