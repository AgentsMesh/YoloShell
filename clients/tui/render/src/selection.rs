use crate::{PaneViewport, constants::*};
use ratatui::{Frame, style::Style};
use uuid::Uuid;

/// Inclusive screen-cell coordinates. The owner validates the selection's
/// runtime, output cursor and viewport before requesting this visual overlay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaneSelection {
    pub pane: Uuid,
    pub start: (u16, u16),
    pub end: (u16, u16),
}

pub fn highlight_selection(frame: &mut Frame<'_>, view: PaneViewport, selection: PaneSelection) {
    let inner = view.inner;
    if selection.pane != view.pane || selection.start > selection.end {
        return;
    }
    // A viewport can outlive an outer-terminal resize. Iterate the intersection
    // in absolute coordinates so clipping also bounds every buffer lookup.
    let visible = inner.intersection(frame.area());
    let style = Style::default()
        .fg(SELECTION_FOREGROUND)
        .bg(SELECTION_BACKGROUND);
    for y in visible.top()..visible.bottom() {
        for x in visible.left()..visible.right() {
            let point = (y - inner.y, x - inner.x);
            if point < selection.start || point > selection.end {
                continue;
            }
            frame.buffer_mut()[(x, y)].set_style(style);
        }
    }
}
