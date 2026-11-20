use super::{drawing_support, fixtures};
use crate::{Mode, network::Job, tabs};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_tui_inventory::View;
use yolo_tui_tab::MAX_TABS;

const SAVED_FILTER: &str = "selected-agent";
const PICKER_FILTER: &str = "detached-terminal";
const FIRST_EXTRA_TAB: usize = 1;
const EXPECTED_TABS: usize = 2;

#[test]
fn missing_tab_activation_preserves_current_layout_and_input_authority() {
    let (mut state, address) = drawing_support::state();
    let layout = state.tab_layout().clone();
    let catalog = state.tab_snapshot();
    let grant = state.writers[&address].grant;
    state.mode = Mode::Inventory;
    assert!(!tabs::activate(&mut state, Uuid::new_v4()));
    assert_eq!(state.mode, Mode::Inventory);
    assert_eq!(*state.tab_layout(), layout);
    assert_eq!(state.tabs.active().id, catalog.active().id);
    assert_eq!(state.writers[&address].grant, grant);
    assert!(!state.status.is_empty());
}

#[test]
fn full_tab_catalog_rejects_open_without_displacing_the_live_terminal() {
    let (mut state, address) = drawing_support::state();
    let active = state.tabs.active().id;
    let layout = state.tab_layout().clone();
    let grant = state.writers[&address].grant;
    state.tabs = state.tab_snapshot();
    for number in FIRST_EXTRA_TAB..MAX_TABS {
        state
            .tabs
            .create(&format!("Reserved tab {number}"))
            .unwrap();
    }
    assert!(tabs::activate(&mut state, active));
    let catalog = state.tab_snapshot();
    let other = fixtures::session();
    let other_address = other.fields().address;
    state.terminals.insert(other_address, other);
    let (jobs, mut pending) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    tabs::open(&mut state, &jobs, other_address);
    assert_eq!(state.tab_snapshot(), catalog);
    assert_eq!(*state.tab_layout(), layout);
    assert_eq!(state.focused_terminal(), Some(address));
    assert_eq!(state.writers[&address].grant, grant);
    assert!(!state.bound_terminals().contains(&other_address));
    assert!(pending.try_recv().is_err());
}

#[test]
fn reopening_the_focused_terminal_does_not_allocate_or_reacquire_a_healthy_writer() {
    let (mut state, address) = drawing_support::state();
    let layout = state.tab_layout().clone();
    let catalog = state.tab_snapshot();
    state.mode = Mode::Inventory;
    let (jobs, mut pending) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    tabs::open(&mut state, &jobs, address);
    assert_eq!(state.mode, Mode::Terminal);
    assert_eq!(state.selected, Some(address));
    assert_eq!(*state.tab_layout(), layout);
    assert_eq!(state.tab_snapshot(), catalog);
    assert!(pending.try_recv().is_err());
}

#[test]
fn opening_existing_terminal_cannot_steal_an_empty_pane_reserved_for_creation() {
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    let other = fixtures::session();
    let address = other.fields().address;
    state.terminals.insert(address, other);
    let (jobs, mut pending) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    assert!(crate::shell_actions::create(
        &mut state,
        &jobs,
        address.hub_id,
        fixtures::SESSION_DIRECTORY.into(),
        None
    ));
    assert!(matches!(pending.try_recv().unwrap(), Job::Call { .. }));
    let reserved = state.tabs.active().id;
    tabs::open(&mut state, &jobs, address);
    assert_ne!(state.tabs.active().id, reserved);
    assert_eq!(state.tabs.entries().len(), EXPECTED_TABS);
    assert_eq!(state.focused_terminal(), Some(address));
    assert!(
        state
            .creations
            .pending
            .values()
            .all(|creation| creation.tab == reserved)
    );
    assert!(
        state
            .tabs
            .entries()
            .iter()
            .find(|entry| entry.id == reserved)
            .unwrap()
            .view
            .layout
            .panes()
            .iter()
            .all(|pane| pane.terminal.is_none())
    );
    while let Ok(job) = pending.try_recv() {
        assert!(matches!(job, Job::Acquire { .. }));
    }
}

#[test]
fn cancelling_the_picker_restores_the_covered_mode_with_a_focused_terminal() {
    let (mut state, _) = drawing_support::state();
    assert!(state.focused_terminal().is_some());
    state.mode = Mode::Inventory;
    tabs::terminal_picker::open(&mut state);
    assert_eq!(state.mode, Mode::Inventory);
    tabs::terminal_picker::close(&mut state);
    assert_eq!(state.mode, Mode::Inventory);
}

#[test]
fn nested_picker_open_preserves_original_sidebar_filter_and_never_changes_bindings() {
    for bound in [false, true] {
        let mut state = if bound {
            drawing_support::state().0
        } else {
            crate::client_capability_support::state()
        };
        state.inventory.view = View::Tree;
        state.filter = SAVED_FILTER.into();
        let mode = state.mode;
        let layout = state.tab_layout().clone();
        let bindings = state.bound_terminals();
        tabs::terminal_picker::open(&mut state);
        assert_eq!(state.inventory.view, View::Terminals);
        assert!(state.filter.is_empty());
        state.filter = PICKER_FILTER.into();
        tabs::terminal_picker::open(&mut state);
        assert_eq!(state.filter, PICKER_FILTER);
        tabs::terminal_picker::close(&mut state);
        assert_eq!(state.inventory.view, View::Tree);
        assert_eq!(state.filter, SAVED_FILTER);
        assert_eq!(state.mode, mode);
        assert_eq!(*state.tab_layout(), layout);
        assert_eq!(state.bound_terminals(), bindings);
        assert!(state.terminal_picker.is_none());
    }
}
