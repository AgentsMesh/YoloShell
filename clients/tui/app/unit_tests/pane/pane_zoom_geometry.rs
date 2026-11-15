use super::panes;
use crate::{pane_actions, pane_view::ZOOM_KEY, pointer_drag};
use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use yolo_hub_terminal_protocol::TerminalSize;

#[test]
fn zoom_cancels_an_existing_divider_drag_and_hides_all_underlying_divider_targets() {
    let (mut state, _, _) = panes::state();
    let divider = state
        .tab_layout()
        .split_geometry(state.presentation.geometry_bounds())
        .unwrap()
        .remove(usize::default());
    let x = divider.boundary;
    let y = divider.rect.y + panes::BORDER;
    assert!(pointer_drag::handle(
        &mut state,
        panes::mouse(MouseEventKind::Down(MouseButton::Left), x, y)
    ));
    assert!(state.pointer.drag.is_some());
    pane_actions::apply(&mut state, KeyCode::Char(ZOOM_KEY))
        .unwrap()
        .unwrap();
    panes::repaint(&mut state);
    let owner = state.tab_layout().clone();
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Drag(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        assert!(!pointer_drag::handle(
            &mut state,
            panes::mouse(kind, x + panes::MOVE_CELLS, y)
        ));
        assert!(state.pointer.drag.is_none());
        assert_eq!(*state.tab_layout(), owner);
    }
}

#[test]
fn zoom_draws_only_focused_geometry_and_waits_for_its_expanded_snapshot() {
    let (mut state, first, focused) = panes::state();
    panes::writable(&mut state, &first, &[]);
    panes::writable(&mut state, &focused, &[]);
    let focused_address = focused.fields().address;
    let hidden_address = first.fields().address;
    let focused_id = state.tab_layout().focused();
    let owner = state.tab_layout().clone();
    let hidden_size = state.last_size(hidden_address).expect("last size");
    let old_size = state.last_size(focused_address).expect("last size");
    pane_actions::apply(&mut state, KeyCode::Char(ZOOM_KEY))
        .unwrap()
        .unwrap();
    panes::repaint(&mut state);
    let view = state.presentation.panes.first().unwrap();
    assert_eq!(state.presentation.panes.len(), SINGLE_VIEWPORT);
    assert_eq!(view.pane, focused_id);
    assert_eq!(view.outer, state.presentation.bounds);
    let desired = TerminalSize::new(view.inner.width, view.inner.height).unwrap();
    assert!(desired.cols() > old_size.cols());
    assert_eq!(
        state.focused_terminal_size(state.presentation.bounds),
        Some(desired)
    );
    assert!(!state.presentation_ready(focused_address, Some(focused_address), Some(desired)));
    assert_eq!(
        state.last_size(hidden_address).expect("last size"),
        hidden_size
    );
    assert_eq!(
        state.surfaces[&hidden_address].screen().size(),
        (hidden_size.rows(), hidden_size.cols())
    );
    panes::writable(&mut state, &focused, &[]);
    assert!(state.presentation_ready(focused_address, Some(focused_address), Some(desired)));
    pane_actions::apply(&mut state, KeyCode::Char(ZOOM_KEY))
        .unwrap()
        .unwrap();
    panes::repaint(&mut state);
    assert_eq!(state.presentation.panes.len(), owner.panes().len());
    assert_eq!(*state.tab_layout(), owner);
    assert_eq!(
        state.last_size(hidden_address).expect("last size"),
        hidden_size
    );
}

const SINGLE_VIEWPORT: usize = 1;
