use super::{panes, selection_support::*};
use crate::{pointer, selection_actions};
use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};

#[test]
fn reverse_drag_and_shift_release_keep_original_local_selection() {
    let mut fixture = Fixture::new();
    fixture.begin(LAST_ALPHA, FIRST);
    let drag = panes::mouse(
        MouseEventKind::Drag(MouseButton::Left),
        fixture.view.inner.x,
        fixture.view.inner.y,
    );
    pointer::handle(&mut fixture.state, &fixture.jobs, drag);
    fixture.finish(FIRST, FIRST);
    fixture.copy();
    assert_eq!(
        selection_actions::take_clipboard(&mut fixture.state)
            .unwrap()
            .as_bytes(),
        EXPECTED_COPY
    );
    fixture.no_input();
}

#[test]
fn dragging_outside_the_pane_clamps_to_its_original_visible_snapshot() {
    let mut fixture = Fixture::new();
    fixture.begin(LAST_ALPHA, FIRST);
    pointer::handle(
        &mut fixture.state,
        &fixture.jobs,
        panes::mouse(MouseEventKind::Up(MouseButton::Left), FIRST, FIRST),
    );
    fixture.copy();
    assert_eq!(
        selection_actions::take_clipboard(&mut fixture.state)
            .unwrap()
            .as_bytes(),
        EXPECTED_COPY
    );
    fixture.no_input();
}

#[test]
fn selecting_another_visible_pane_changes_only_local_focus_and_keeps_the_drag() {
    let (mut state, first, _) = panes::state();
    panes::writable(&mut state, &first, TEXT);
    state.writers.clear();
    let view = state
        .presentation
        .panes
        .iter()
        .find(|view| view.terminal == Some(first.fields().address))
        .copied()
        .unwrap();
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let mut down = panes::mouse(
        MouseEventKind::Down(MouseButton::Left),
        view.inner.x,
        view.inner.y,
    );
    down.modifiers = KeyModifiers::SHIFT;
    pointer::handle(&mut state, &jobs, down);
    selection_actions::sync(&mut state);
    assert_eq!(state.tab_layout().focused(), view.pane);
    assert!(state.selection.active.as_ref().unwrap().text.is_dragging());
    panes::repaint(&mut state);
    pointer::handle(
        &mut state,
        &jobs,
        panes::mouse(
            MouseEventKind::Up(MouseButton::Left),
            view.inner.x + LAST_ALPHA,
            view.inner.y,
        ),
    );
    selection_actions::copy(&mut state);
    assert_eq!(
        selection_actions::take_clipboard(&mut state)
            .unwrap()
            .as_bytes(),
        EXPECTED_COPY
    );
    assert!(incoming.try_recv().is_err());
    assert!(state.writers.is_empty());
}

#[test]
fn shift_press_on_border_or_missing_surface_does_not_fall_through_to_provider_input() {
    for missing in [false, true] {
        let mut fixture = Fixture::new();
        if missing {
            fixture.state.surfaces.clear();
        }
        let mut down = panes::mouse(
            MouseEventKind::Down(MouseButton::Left),
            fixture.view.outer.x,
            fixture.view.outer.y,
        );
        if missing {
            down.column = fixture.view.inner.x;
            down.row = fixture.view.inner.y;
        }
        down.modifiers = KeyModifiers::SHIFT;
        pointer::handle(&mut fixture.state, &fixture.jobs, down);
        assert!(fixture.state.selection.active.is_none());
        fixture.no_input();
    }
}
