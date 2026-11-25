use crate::{Mode, State};
use yolo_tui_inventory::View;

#[derive(Clone)]
pub(crate) struct TerminalPicker {
    view: View,
    filter: String,
    mode: Mode,
}

pub(crate) fn open(state: &mut State) {
    if state.terminal_picker.is_some() {
        return;
    }
    state.terminal_picker = Some(TerminalPicker {
        view: state.inventory.view,
        filter: std::mem::take(&mut state.filter),
        mode: state.mode,
    });
    state.inventory.view = View::Terminals;
    state.inventory.details = None;
    state.mode = Mode::Inventory;
    state.selected = state.focused_terminal().or(state.selected);
    state.reconcile_selection();
    state.presentation = Default::default();
}

pub(crate) fn close(state: &mut State) {
    if let Some(picker) = state.terminal_picker.take() {
        state.inventory.view = picker.view;
        state.filter = picker.filter;
        // Cancelling the overlay returns to the view it covered. Selecting a
        // terminal goes through the activation path, which sets Terminal
        // after this restore.
        state.mode = picker.mode;
        // The chat that existed when the picker opened may have been retired
        // by a catalogue refresh while the overlay covered it.
        super::settle_chat_mode(state);
        state.presentation = Default::default();
    }
}
