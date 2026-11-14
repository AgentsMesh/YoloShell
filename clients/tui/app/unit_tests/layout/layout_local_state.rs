use super::{fixtures, layout_support::*};
use crate::layout_sync::refresh;
use yolo_presentation_layout::{Pane, Tree};

#[test]
fn remote_rename_retains_client_modal_state_and_zoom() {
    let (mut state, jobs, mut incoming) = fixture();
    state.mode = crate::Mode::Directory;
    state.entry = "/unfinished/local/input".into();
    state.pane_view.zoomed = true;
    state.prefix = true;
    let mut changed = remote(&state);
    changed.tabs[0].title = "another client renamed this tab".into();
    refresh(&mut state);
    let operation = request(&mut state, &jobs, &mut incoming);
    response(&mut state, operation, changed);
    assert_eq!(state.mode, crate::Mode::Directory);
    assert_eq!(state.entry, "/unfinished/local/input");
    assert!(state.pane_view.zoomed);
    assert!(state.prefix);
}

#[test]
fn rebinding_the_same_pane_cancels_local_modal_and_does_not_grant_input() {
    let (mut state, jobs, mut incoming) = fixture();
    let target = fixtures::session();
    let address = target.fields().address;
    let focused = state.tab_layout().focused();
    state.mode = crate::Mode::ConfirmTerminate;
    state.pane_view.zoomed = true;
    state.prefix = true;
    let mut changed = remote(&state);
    changed.tabs[0].root = Tree::Pane(Pane {
        id: focused,
        terminal: Some(address),
    });
    refresh(&mut state);
    let operation = request(&mut state, &jobs, &mut incoming);
    response(&mut state, operation, changed);
    assert_eq!(state.tab_layout().focused(), focused);
    assert_eq!(state.focused_terminal(), Some(address));
    assert_eq!(state.mode, crate::Mode::Inventory);
    assert!(!state.prefix);
    assert!(!state.pane_view.zoomed);
    assert!(state.writers.is_empty());
    assert!(incoming.try_recv().is_err());
}
