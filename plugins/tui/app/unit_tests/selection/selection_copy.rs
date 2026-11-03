use super::selection_support::*;
use crate::selection_actions;
use crossterm::event::{KeyCode, KeyModifiers};

#[test]
fn explicit_copy_works_for_observers_even_with_provider_mouse_mode_and_never_acquires_input() {
    let mut fixture = Fixture::new();
    fixture.begin(FIRST, FIRST);
    assert!(selection_actions::take_clipboard(&mut fixture.state).is_none());
    fixture.finish(LAST_ALPHA, FIRST);
    assert!(selection_actions::take_clipboard(&mut fixture.state).is_none());
    fixture.copy();
    assert_eq!(
        selection_actions::take_clipboard(&mut fixture.state)
            .unwrap()
            .as_bytes(),
        EXPECTED_COPY
    );
    assert!(selection_actions::take_clipboard(&mut fixture.state).is_none());
    assert!(fixture.state.selection.active.is_none());
    fixture.no_input();
}

#[test]
fn copying_without_selection_or_before_release_and_provider_osc_never_emit_clipboard_commands() {
    let mut fixture = Fixture::new();
    fixture.emit(PROVIDER_COPY);
    fixture.copy();
    assert!(selection_actions::take_clipboard(&mut fixture.state).is_none());
    fixture.begin(FIRST, FIRST);
    fixture.copy();
    assert!(selection_actions::take_clipboard(&mut fixture.state).is_none());
    assert!(
        fixture
            .state
            .selection
            .active
            .as_ref()
            .unwrap()
            .text
            .is_dragging()
    );
    fixture.no_input();
}

#[test]
fn unprefixed_copy_key_and_escape_do_not_touch_the_clipboard() {
    for key in [
        KeyCode::Char(crate::selection_state::COPY_KEY),
        KeyCode::Esc,
    ] {
        let mut fixture = Fixture::new();
        fixture.select();
        fixture.key(key, KeyModifiers::NONE);
        assert!(selection_actions::take_clipboard(&mut fixture.state).is_none());
        assert!(fixture.state.selection.active.is_none());
        if key == KeyCode::Esc {
            assert_eq!(fixture.state.status, crate::selection_state::CLEARED_LABEL);
        }
        fixture.no_input();
    }
}

#[test]
fn busy_terminal_continues_updating_while_selection_and_display_copy_the_frozen_text() {
    let mut fixture = Fixture::new();
    fixture.begin(FIRST, FIRST);
    fixture.emit(REPLACEMENT);
    selection_actions::sync(&mut fixture.state);
    fixture.finish(LAST_ALPHA, FIRST);
    let frozen = selection_actions::display_screen(&fixture.state, fixture.target.address).unwrap();
    assert!(frozen.contents().starts_with("alpha"));
    assert_eq!(
        fixture.state.surfaces[&fixture.target.address]
            .screen()
            .contents(),
        "changed"
    );
    assert!(selection_actions::highlight(&fixture.state, fixture.view).is_some());
    fixture.copy();
    assert_eq!(
        selection_actions::take_clipboard(&mut fixture.state)
            .unwrap()
            .as_bytes(),
        EXPECTED_COPY
    );
    assert!(selection_actions::display_screen(&fixture.state, fixture.target.address).is_none());
    fixture.no_input();
}
