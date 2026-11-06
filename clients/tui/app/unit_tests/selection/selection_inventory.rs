use super::{panes, selection_support::*};
use crate::{Mode, selection_actions};
use crossterm::event::{KeyCode, KeyModifiers};

#[test]
fn inventory_escape_resumes_frozen_output_without_clearing_filter_scope_or_layout() {
    let mut fixture = selected_inventory();
    let filter = fixture.state.filter.clone();
    let scope = fixture.state.inventory.scope;
    let layout = fixture.state.tab_layout().clone();
    fixture.emit(REPLACEMENT);
    let current = fixture.state.surfaces[&fixture.target.address]
        .screen()
        .contents();
    assert_ne!(
        selection_actions::display_screen(&fixture.state, fixture.target.address)
            .unwrap()
            .contents(),
        current
    );
    fixture.key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(fixture.state.selection.active.is_none());
    assert!(selection_actions::display_screen(&fixture.state, fixture.target.address).is_none());
    assert!(selection_actions::take_clipboard(&mut fixture.state).is_none());
    assert_eq!(fixture.state.filter, filter);
    assert_eq!(fixture.state.inventory.scope, scope);
    assert_eq!(*fixture.state.tab_layout(), layout);
    assert_eq!(fixture.state.mode, Mode::Inventory);
    fixture.no_input();
    // Once selection is gone, the next Escape retains normal inventory behavior.
    fixture.key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(fixture.state.filter.is_empty());
    assert_eq!(*fixture.state.tab_layout(), layout);
    fixture.no_input();
}

#[test]
fn ordinary_inventory_navigation_resumes_live_terminal_without_copy_or_hub_commands() {
    for key in [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Char(SEARCH_KEY),
    ] {
        let mut fixture = selected_inventory();
        let layout = fixture.state.tab_layout().clone();
        fixture.key(key, KeyModifiers::NONE);
        assert!(fixture.state.selection.active.is_none());
        assert!(
            selection_actions::display_screen(&fixture.state, fixture.target.address).is_none()
        );
        assert!(selection_actions::take_clipboard(&mut fixture.state).is_none());
        assert_eq!(*fixture.state.tab_layout(), layout);
        fixture.no_input();
    }
}

fn selected_inventory() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Inventory;
    fixture.state.filter = fixture.state.terminals[&fixture.target.address]
        .fields()
        .name
        .clone();
    fixture.state.inventory.scope = Some(fixture.target.address.hub_id);
    panes::repaint(&mut fixture.state);
    fixture.view = fixture
        .state
        .presentation
        .panes
        .iter()
        .find(|view| view.pane == fixture.state.tab_layout().focused())
        .copied()
        .unwrap();
    fixture.select();
    assert!(fixture.state.selection.active.is_some());
    assert_eq!(fixture.state.mode, Mode::Inventory);
    fixture.no_input();
    fixture
}

const SEARCH_KEY: char = '/';
