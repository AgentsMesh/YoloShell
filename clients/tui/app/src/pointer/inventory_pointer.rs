use crate::{Mode, State, network::Job};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;
use tokio::sync::mpsc;
use yolo_tui_inventory::{InventoryHit, InventorySelection};

pub(crate) const SELECTED_HINT: &str = "Click/Enter open/details · h history";

/// Inventory gestures select only; a click that selects an Agent row also
/// opens its existing terminal, while a wheel notch only moves selection so
/// the list stays browsable. A click never injects terminal input or retargets
/// a stale frame.
pub(crate) fn handle(state: &mut State, jobs: &mpsc::Sender<Job>, event: MouseEvent) -> bool {
    if !matches!(state.mode, Mode::Inventory | Mode::Terminal | Mode::Chat)
        || !state.presentation.current(&state.tab_layout(), state.mode)
    {
        return false;
    }
    let Some(viewport) = &state.presentation.inventory else {
        return false;
    };
    let position = Position::new(event.column, event.row);
    if !viewport.area().contains(position) {
        return false;
    }
    if !event.modifiers.is_empty()
        || !viewport.current(
            &state.inventory,
            &state.terminals,
            state.selected,
            &state.filter,
        )
    {
        return true;
    }
    let (selection, clicked) = match event.kind {
        MouseEventKind::Down(MouseButton::Left) => match viewport.hit(event.column, event.row) {
            Some(InventoryHit::Tab(view)) => {
                state.mode = Mode::Inventory;
                state.inventory.view = view;
                state.inventory.details = None;
                state.reconcile_selection();
                state.status = SELECTED_HINT.into();
                return true;
            }
            Some(InventoryHit::Row(selection)) => (Some(selection), true),
            None => (None, false),
        },
        MouseEventKind::ScrollDown if viewport.rows_area().contains(position) => {
            (viewport.scroll_target(true), false)
        }
        MouseEventKind::ScrollUp if viewport.rows_area().contains(position) => {
            (viewport.scroll_target(false), false)
        }
        _ => (None, false),
    };
    if let Some(selection) = selection {
        state.mode = Mode::Inventory;
        if state.inventory.select(
            selection,
            &mut state.selected,
            &state.filter,
            &state.terminals,
        ) {
            state.inventory.details = None;
            state.status = SELECTED_HINT.into();
            if clicked && matches!(selection, InventorySelection::Agent(_)) {
                crate::inventory_actions::open(state, jobs);
            }
        }
    }
    true
}
