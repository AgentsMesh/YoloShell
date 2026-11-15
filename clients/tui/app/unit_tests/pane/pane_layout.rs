use super::panes::*;
use crate::{Mode, pane_actions, pointer};
use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use yolo_presentation_layout::{Axis, SplitRatio};

#[test]
fn keyboard_direction_and_ratio_controls_change_layout_without_control_jobs() {
    let (mut state, _, _) = state();
    let second = state.tab_layout().focused();
    pane_actions::apply(&mut state, KeyCode::Left)
        .unwrap()
        .unwrap();
    assert_ne!(state.tab_layout().focused(), second);
    repaint(&mut state);
    pane_actions::apply(&mut state, KeyCode::Char('L'))
        .unwrap()
        .unwrap();
    let ratio = state
        .tab_layout()
        .split_geometry(state.presentation.geometry_bounds())
        .unwrap()[0]
        .ratio;
    assert!(u16::from(ratio) > u16::from(SplitRatio::default()));
    pane_actions::apply(&mut state, KeyCode::Char('='))
        .unwrap()
        .unwrap();
    assert_eq!(
        state
            .tab_layout()
            .split_geometry(state.presentation.geometry_bounds())
            .unwrap()[0]
            .ratio,
        SplitRatio::default()
    );
    assert!(pane_actions::apply(&mut state, KeyCode::Char('!')).is_none());
    assert!(state.writers.is_empty());
}

#[test]
fn border_drag_is_relative_preserves_identity_and_a_click_is_not_a_resize() {
    let (mut state, _, _) = state();
    let original = state.tab_layout().clone();
    let split = state
        .tab_layout()
        .split_geometry(state.presentation.geometry_bounds())
        .unwrap()
        .remove(0);
    let x = split.boundary;
    let y = split.rect.y + BORDER;
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::Down(MouseButton::Left), x, y),
    );
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::Up(MouseButton::Left), x, y),
    );
    assert_eq!(*state.tab_layout(), original);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::Down(MouseButton::Left), x, y),
    );
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::Drag(MouseButton::Left), x + MOVE_CELLS, y),
    );
    assert_ne!(*state.tab_layout(), original);
    assert_eq!(state.tab_layout().panes(), original.panes());
    assert_eq!(state.tab_layout().focused(), original.focused());
    repaint(&mut state);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::Up(MouseButton::Left), x, y),
    );
    assert_eq!(*state.tab_layout(), original);
    assert!(state.pointer.drag.is_none());
    assert!(incoming.try_recv().is_err());
}

#[test]
fn replaced_layout_mode_or_window_geometry_cancels_an_old_drag() {
    for change in 0..INVALIDATIONS {
        let (mut state, _, _) = state();
        let split = state
            .tab_layout()
            .split_geometry(state.presentation.geometry_bounds())
            .unwrap()
            .remove(0);
        let (x, y) = (split.boundary, split.rect.y + BORDER);
        let (jobs, _) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
        pointer::handle(
            &mut state,
            &jobs,
            mouse(MouseEventKind::Down(MouseButton::Left), x, y),
        );
        match change {
            0 => {
                state.tab_layout_mut().split(Axis::Vertical, None).unwrap();
            }
            1 => state.mode = Mode::Inventory,
            _ => state.presentation.bounds.width += BORDER,
        }
        let expected = state.tab_layout().clone();
        pointer::handle(
            &mut state,
            &jobs,
            mouse(MouseEventKind::Drag(MouseButton::Left), x + MOVE_CELLS, y),
        );
        assert_eq!(*state.tab_layout(), expected);
        assert!(state.pointer.drag.is_none());
    }
}
const INVALIDATIONS: usize = 3;
