use super::panes;
use crate::{
    pane_actions,
    pane_view::{SWAP_NEXT_KEY, SWAP_PREVIOUS_KEY, ZOOM_KEY},
};
use crossterm::event::KeyCode;
use std::collections::BTreeMap;

#[test]
fn zoom_preserves_the_owner_layout_background_bindings_writers_and_observers() {
    let (mut state, first, second) = panes::state();
    panes::writable(&mut state, &first, &[]);
    panes::writable(&mut state, &second, &[]);
    for session in [&first, &second] {
        state.set_observer_generation(session.fields().address, uuid::Uuid::new_v4());
    }
    let owner = state.tab_layout().clone();
    let writers = state
        .writers
        .iter()
        .map(|(address, writer)| (*address, writer.grant))
        .collect::<BTreeMap<_, _>>();
    let observers = state.observer_generation_snapshot();
    let sizes = state.last_size_snapshot();
    pane_actions::apply(&mut state, KeyCode::Char(ZOOM_KEY))
        .unwrap()
        .unwrap();
    assert!(state.pane_view.zoomed);
    assert_eq!(&*state.display_layout(), &owner.focused_view());
    assert_eq!(*state.tab_layout(), owner);
    assert_eq!(state.observer_generation_snapshot(), observers);
    assert_eq!(state.last_size_snapshot(), sizes);
    assert_eq!(
        state
            .writers
            .iter()
            .map(|(address, writer)| (*address, writer.grant))
            .collect::<BTreeMap<_, _>>(),
        writers
    );
    pane_actions::apply(&mut state, KeyCode::Char(ZOOM_KEY))
        .unwrap()
        .unwrap();
    assert!(!state.pane_view.zoomed);
    assert_eq!(&*state.display_layout(), &owner);
    assert!(matches!(
        state.display_layout(),
        std::borrow::Cow::Borrowed(_)
    ));
}

#[test]
fn focused_identity_survives_swap_while_zoom_follows_explicit_focus_changes() {
    let (mut state, _, _) = panes::state();
    let original = state.tab_layout().clone();
    let focused = original.focused();
    let session = state.focused_terminal();
    pane_actions::apply(&mut state, KeyCode::Char(ZOOM_KEY))
        .unwrap()
        .unwrap();
    pane_actions::apply(&mut state, KeyCode::Char(SWAP_PREVIOUS_KEY))
        .unwrap()
        .unwrap();
    assert_eq!(state.tab_layout().focused(), focused);
    assert_eq!(state.focused_terminal(), session);
    assert_eq!(state.display_layout().focused(), focused);
    assert_eq!(state.tab_layout().panes().first().unwrap().id, focused);
    pane_actions::apply(&mut state, KeyCode::Char(SWAP_NEXT_KEY))
        .unwrap()
        .unwrap();
    assert_eq!(*state.tab_layout(), original);
    let other = state
        .tab_layout()
        .panes()
        .iter()
        .find(|pane| pane.id != focused)
        .unwrap()
        .id;
    state.tab_layout_mut().focus(other).unwrap();
    assert!(state.pane_view.zoomed);
    assert_eq!(state.display_layout().focused(), other);
    assert_eq!(state.display_layout().panes().first().unwrap().id, other);
}

#[test]
fn single_pane_zoom_and_swap_are_stable_noops() {
    let mut state = crate::client_capability_support::state();
    let original = state.tab_layout().clone();
    for key in [ZOOM_KEY, SWAP_PREVIOUS_KEY, SWAP_NEXT_KEY] {
        pane_actions::apply(&mut state, KeyCode::Char(key))
            .unwrap()
            .unwrap();
        assert_eq!(*state.tab_layout(), original);
        assert!(!state.pane_view.zoomed);
    }
}
