use crate::State;
use ratatui::{Frame, layout::Rect};
use yolo_tui_render::{TabHit, TabStripItem, draw_tab_strip};

const FIRST_TAB_NUMBER: usize = 1;
const ACTIVE_MARKER: &str = "●";
const INACTIVE_MARKER: &str = " ";
const TAB_PADDING: &str = " ";
const ENDED_TAB_MARKER: &str = "×";
const LOST_TAB_MARKER: &str = "!";

pub(crate) fn draw(frame: &mut Frame<'_>, state: &State, area: Rect) -> Vec<TabHit> {
    let active = state.tabs.active().id;
    let items = state
        .tabs
        .machine_entries()
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let marker = if entry.id == active {
                ACTIVE_MARKER
            } else {
                INACTIVE_MARKER
            };
            let number = index + FIRST_TAB_NUMBER;
            TabStripItem {
                id: entry.id,
                label: format!(
                    "{TAB_PADDING}{marker}{number} {}{TAB_PADDING}",
                    title(state, entry)
                ),
                active: entry.id == active,
            }
        })
        .collect::<Vec<_>>();
    draw_tab_strip(frame, area, &items)
}

fn title(state: &State, entry: &yolo_tui_tab::TabEntry) -> String {
    let layout = if entry.id == state.tabs.active().id {
        state.tab_layout()
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
    let phase_marker = address.and_then(|address| {
        match crate::terminal_lifecycle::phase(state, address) {
            yolo_tui_lifecycle::TerminalPhase::Ended
            | yolo_tui_lifecycle::TerminalPhase::Archived => Some(ENDED_TAB_MARKER),
            yolo_tui_lifecycle::TerminalPhase::Lost => Some(LOST_TAB_MARKER),
            _ => None,
        }
    });
    if let Some(marker) = phase_marker {
        return format!("{marker} {title}");
    }
    let control_marker = address.and_then(|address| crate::control::marker(state, address));
    control_marker.map_or(title.clone(), |marker| format!("{marker} {title}"))
}
