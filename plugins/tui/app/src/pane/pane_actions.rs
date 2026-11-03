use crate::State;
use crate::pane_view::{SWAP_NEXT_KEY, SWAP_PREVIOUS_KEY, ZOOM_KEY};
use crossterm::event::KeyCode;
use yolo_presentation_layout::{Axis, Direction, LayoutError, SwapDirection};

pub fn apply(state: &mut State, key: KeyCode) -> Option<Result<(), LayoutError>> {
    if key == KeyCode::Char(ZOOM_KEY) {
        state.toggle_pane_zoom();
        return Some(Ok(()));
    }
    let swap = match key {
        KeyCode::Char(SWAP_PREVIOUS_KEY) => Some(SwapDirection::Previous),
        KeyCode::Char(SWAP_NEXT_KEY) => Some(SwapDirection::Next),
        _ => None,
    };
    if let Some(direction) = swap {
        state.pointer = Default::default();
        return Some(state.tab_layout_mut().swap_focused(direction).map(|_| ()));
    }
    let direction = match key {
        KeyCode::Left => Some(Direction::Left),
        KeyCode::Right => Some(Direction::Right),
        KeyCode::Up => Some(Direction::Up),
        KeyCode::Down => Some(Direction::Down),
        _ => None,
    };
    if let Some(direction) = direction {
        if !state.presentation.current(&state.tab_layout(), state.mode) {
            return Some(Ok(()));
        }
        let bounds = state.presentation.geometry_bounds();
        return Some(
            state
                .tab_layout_mut()
                .focus_direction(direction, bounds)
                .map(|_| ()),
        );
    }
    let adjust = match key {
        KeyCode::Char('H') => Some((Axis::Horizontal, false)),
        KeyCode::Char('L') => Some((Axis::Horizontal, true)),
        KeyCode::Char('K') => Some((Axis::Vertical, false)),
        KeyCode::Char('J') => Some((Axis::Vertical, true)),
        _ => None,
    };
    if let Some((axis, grow)) = adjust {
        return Some(
            state
                .tab_layout_mut()
                .adjust_focused(axis, grow, crate::constants::PANE_RATIO_STEP)
                .map(|_| ()),
        );
    }
    if key == KeyCode::Char('=') {
        state.tab_layout_mut().balance();
        return Some(Ok(()));
    }
    None
}
