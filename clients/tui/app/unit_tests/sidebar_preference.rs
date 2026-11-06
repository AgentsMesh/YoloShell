use super::{drawing_support, fixtures};
use crate::{Mode, tabs};
use yolo_tui_inventory::View;

const COLUMNS: u16 = 80;
const ROWS: u16 = 24;

#[test]
fn default_shell_is_full_width_and_catalog_changes_do_not_open_the_sidebar() {
    let (mut state, _) = drawing_support::state();
    let (agent, terminal) = fixtures::agent();
    for agents in [vec![agent], Vec::new()] {
        state.inventory.replace(Some(agents), None);
        state
            .terminals
            .insert(terminal.fields().address, terminal.clone());
        let (_, presentation) = drawing_support::render(&state, COLUMNS, ROWS);
        assert!(!state.sidebar_visible);
        assert!(presentation.inventory.is_none());
        assert_eq!(presentation.bounds.width, COLUMNS);
    }
}

#[test]
fn explicitly_open_sidebar_stays_visible_when_empty_and_when_terminal_regains_focus() {
    let (mut state, _) = drawing_support::state();
    tabs::sidebar(&mut state);
    state.inventory.replace(Some(Vec::new()), None);
    state.inventory.view = View::Tree;
    let (_, sidebar) = drawing_support::render(&state, COLUMNS, ROWS);
    assert!(state.sidebar_visible);
    assert!(sidebar.inventory.is_some());
    state.mode = Mode::Terminal;
    let (_, terminal) = drawing_support::render(&state, COLUMNS, ROWS);
    assert!(state.sidebar_visible);
    assert_eq!(terminal.bounds, sidebar.bounds);
}

#[test]
fn an_empty_layout_uses_inventory_width_without_forgetting_sidebar_preference() {
    let mut state = crate::client_capability_support::state();
    tabs::sidebar(&mut state);
    let (_, view) = drawing_support::render(&state, COLUMNS, ROWS);
    assert!(state.sidebar_visible);
    assert!(!view.bounds.is_empty());
    assert!(view.inventory.unwrap().area().width < COLUMNS);
}

#[test]
fn restored_inventory_focus_is_visible_alongside_its_bound_terminal() {
    let (mut state, address) = drawing_support::state();
    assert!(!state.sidebar_visible);
    state.mode = Mode::Inventory;
    let (_, view) = drawing_support::render(&state, COLUMNS, ROWS);
    assert!(view.inventory.is_some());
    assert!(view.panes.iter().any(|pane| pane.terminal == Some(address)));
    assert!(!view.bounds.is_empty());
    // Focus is a visibility requirement, not a change to the user's terminal-mode preference.
    assert!(!state.sidebar_visible);
    state.mode = Mode::Terminal;
    let (_, terminal) = drawing_support::render(&state, COLUMNS, ROWS);
    assert!(terminal.inventory.is_none());
    assert_eq!(terminal.bounds.width, COLUMNS);
}
