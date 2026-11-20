use crate::{State, selection_state::*};
use yolo_hub_identity::TerminalAddress;
use yolo_tui_render::{PaneSelection, PaneViewport};
use yolo_tui_selection::ClipboardCommand;

pub(crate) fn cancel(state: &mut State) {
    if state.selection.active.take().is_some() {
        state.status = CLEARED_LABEL.into();
    }
}

pub(crate) fn sync(state: &mut State) {
    if state
        .selection
        .active
        .as_ref()
        .is_some_and(|active| !active.source.current(state))
    {
        cancel(state);
    }
}

/// Call only from the explicit Ctrl+B y key-press branch.
pub(crate) fn copy(state: &mut State) {
    sync(state);
    let Some(active) = state.selection.active.as_ref() else {
        state.status = NO_SELECTION_LABEL.into();
        return;
    };
    match active.text.clipboard() {
        Ok(command) => {
            // A single bounded command waits for the outer screen's owner.
            state.selection.pending = Some(command);
            cancel(state);
            state.status = COPY_QUEUED_LABEL.into();
        }
        Err(error) => state.status = error.to_string(),
    }
}

pub(crate) fn take_clipboard(state: &mut State) -> Option<ClipboardCommand> {
    state.selection.pending.take()
}

/// New provider output continues on the live surface while this image is shown.
pub(crate) fn display_screen(state: &State, address: TerminalAddress) -> Option<&vt100::Screen> {
    let active = state.selection.active.as_ref()?;
    (active.source.target.address == address && active.source.current(state))
        .then(|| active.text.screen())
}

pub(crate) fn highlight(state: &State, view: PaneViewport) -> Option<PaneSelection> {
    let active = state.selection.active.as_ref()?;
    if view.pane != active.source.pane
        || view.inner != active.source.inner
        || !active.source.current(state)
    {
        return None;
    }
    let (start, end) = active.text.range();
    Some(PaneSelection {
        pane: view.pane,
        start: (start.row, start.column),
        end: (end.row, end.column),
    })
}
