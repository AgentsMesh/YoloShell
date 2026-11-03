use crate::{Mode, State, selection_actions, selection_state::*};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use yolo_tui_selection::{CellPoint, Selection};

const CELL_STEP: u16 = 1;

/// Run before provider mouse capture/forwarding. No network channel is accepted.
pub(crate) fn handle(state: &mut State, event: MouseEvent) -> bool {
    selection_actions::sync(state);
    if !matches!(state.mode, Mode::Inventory | Mode::Terminal) {
        selection_actions::cancel(state);
        return false;
    }
    let start = event.kind == MouseEventKind::Down(MouseButton::Left)
        && event.modifiers.contains(KeyModifiers::SHIFT);
    if start {
        state.pointer = crate::pointer::Pointer::default();
        selection_actions::cancel(state);
        begin(state, event);
        return true;
    }
    let dragging = state
        .selection
        .active
        .as_ref()
        .is_some_and(|active| active.text.is_dragging());
    if dragging {
        match event.kind {
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left) => {
                update(state, event);
            }
            MouseEventKind::Down(_) => selection_actions::cancel(state),
            _ => {}
        }
        return true;
    }
    if matches!(event.kind, MouseEventKind::Down(_)) {
        selection_actions::cancel(state);
    }
    false
}

fn begin(state: &mut State, event: MouseEvent) {
    if !state.presentation.current(&state.tab_layout(), state.mode) {
        return;
    }
    let Some(view) = state
        .presentation
        .panes
        .iter()
        .find(|view| crate::pointer::contains(view.inner, event.column, event.row))
        .copied()
    else {
        return;
    };
    let Some(surface) = view
        .terminal
        .and_then(|address| state.surfaces.get(&address))
    else {
        return;
    };
    if !crate::observation_events::current(state, surface.cursor().target)
        || state.tab_manager.is_some()
        || state.hub_manager.is_some()
        || state.presentation.zoomed != state.pane_view.zoomed
    {
        return;
    }
    let (rows, columns) = surface.screen().size();
    let selection = Selection::begin(
        surface.screen(),
        view.inner.height.min(rows),
        view.inner.width.min(columns),
        CellPoint {
            row: event.row - view.inner.y,
            column: event.column - view.inner.x,
        },
    );
    let text = match selection {
        Ok(text) => text,
        Err(error) => {
            state.status = error.to_string();
            return;
        }
    };
    let target = surface.cursor().target;
    let scrollback = surface.screen().scrollback();
    if let Err(error) = state.tab_layout_mut().focus(view.pane) {
        state.status = error.to_string();
        return;
    }
    state.selection.active = Some(BoundSelection {
        source: Source {
            pane: view.pane,
            target,
            inner: view.inner,
            size: (rows, columns),
            scrollback,
            layout: state.tab_layout().clone(),
            mode: state.mode,
            zoomed: state.pane_view.zoomed,
        },
        text,
    });
    state.status = SELECTING_LABEL.into();
}

fn update(state: &mut State, event: MouseEvent) {
    let Some(active) = state.selection.active.as_mut() else {
        return;
    };
    let (rows, columns) = active.text.dimensions();
    let point = CellPoint {
        row: event
            .row
            .saturating_sub(active.source.inner.y)
            .min(rows - CELL_STEP),
        column: event
            .column
            .saturating_sub(active.source.inner.x)
            .min(columns - CELL_STEP),
    };
    let result = if event.kind == MouseEventKind::Up(MouseButton::Left) {
        active.text.finish(point)
    } else {
        active.text.extend(point)
    };
    match result {
        Ok(()) if !active.text.is_dragging() => state.status = SELECTED_LABEL.into(),
        Ok(()) => {}
        Err(error) => {
            selection_actions::cancel(state);
            state.status = error.to_string();
        }
    }
}
