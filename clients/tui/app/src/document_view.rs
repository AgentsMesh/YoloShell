use crate::{Mode, State};
use crossterm::event::Event;

pub(crate) fn handle(state: &mut State, event: &Event) -> bool {
    if !state.document.is_visible(&state.tabs) {
        return false;
    }
    if state.document.handle(event) {
        close(state);
    }
    true
}
fn close(state: &mut State) {
    state.document.close(&mut state.tabs);
    state.mode = if state.focused_terminal().is_some() {
        Mode::Terminal
    } else {
        Mode::Inventory
    };
    crate::resize::invalidate_layout(state, true);
}
