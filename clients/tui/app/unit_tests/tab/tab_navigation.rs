use super::{drawing_support, fixtures, panes};
use crate::{Mode, State, network::Job, tabs};
use crossterm::event::KeyCode;
use tokio::sync::mpsc;
use yolo_hub_identity::HubId;

const OTHER_TAB_NAME: &str = "Other tab";
const EXPECTED_TABS: usize = 2;
const NARROW_COLUMNS: u16 = 60;

fn background_tab(state: &mut State) {
    state.tabs = state.tab_snapshot();
    state.tabs.create(OTHER_TAB_NAME).unwrap();
    state.activate_tab_collection();
}

fn no_create(pending: &mut mpsc::Receiver<Job>) {
    while let Ok(job) = pending.try_recv() {
        assert!(matches!(job, Job::Acquire { .. } | Job::Call { .. }));
        if let Job::Call { command, .. } = job {
            assert!(!matches!(
                command,
                yolo_hub_rpc_protocol::Command::Create { .. }
            ));
        }
    }
}

#[test]
fn opening_bound_agent_focuses_original_tab_and_pane_without_replacing_any_binding() {
    let (mut state, first, second) = panes::state();
    let original_tab = state.tabs.active().id;
    let original_bindings = state
        .tab_layout()
        .panes()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    let first_pane = original_bindings
        .iter()
        .find(|pane| pane.terminal == Some(first.fields().address))
        .unwrap()
        .id;
    background_tab(&mut state);
    let background = state.tab_layout().clone();
    let (jobs, mut pending) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    tabs::open(&mut state, &jobs, first.fields().address);
    assert_eq!(state.tabs.active().id, original_tab);
    assert_eq!(state.tab_layout().focused(), first_pane);
    assert_eq!(
        state
            .tab_layout()
            .panes()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        original_bindings
    );
    assert_eq!(state.tabs.entries().len(), EXPECTED_TABS);
    assert_eq!(state.tabs.entries().last().unwrap().view.layout, background);
    tabs::open(&mut state, &jobs, second.fields().address);
    assert_eq!(state.tabs.active().id, original_tab);
    assert_eq!(state.focused_terminal(), Some(second.fields().address));
    assert_eq!(state.tabs.entries().len(), EXPECTED_TABS);
    no_create(&mut pending);
}

#[test]
fn opening_unbound_terminal_creates_a_new_tab_without_replacing_the_old_shell() {
    let (mut state, address) = drawing_support::state();
    let original_tab = state.tabs.active().id;
    let original_layout = state.tab_layout().clone();
    let other = fixtures::session();
    let other_address = other.fields().address;
    state.terminals.insert(other_address, other);
    let (jobs, mut pending) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    tabs::open(&mut state, &jobs, other_address);
    assert_ne!(state.tabs.active().id, original_tab);
    assert_eq!(state.tabs.entries().len(), EXPECTED_TABS);
    assert_eq!(state.focused_terminal(), Some(other_address));
    assert_eq!(state.tabs.entries()[0].view.layout, original_layout);
    assert!(state.bound_terminals().contains(&address));
    assert!(state.bound_terminals().contains(&other_address));
    no_create(&mut pending);
}

#[test]
fn sidebar_focus_changes_no_pty_dimensions_or_layout() {
    let (mut state, address) = drawing_support::state();
    state.sidebar_visible = true;
    let original_layout = state.tab_layout().clone();
    let (_, before) = drawing_support::render(&state, panes::WIDTH, panes::HEIGHT);
    drawing_support::synchronize(&mut state, address, drawing_support::size(&before, address));
    tabs::sidebar(&mut state);
    let (_, sidebar) = drawing_support::render(&state, panes::WIDTH, panes::HEIGHT);
    assert_eq!(state.mode, Mode::Inventory);
    assert_eq!(before.bounds, sidebar.bounds);
    assert_eq!(before.panes, sidebar.panes);
    assert_eq!(*state.tab_layout(), original_layout);
    assert!(state.presentation_ready(
        address,
        Some(address),
        Some(drawing_support::size(&sidebar, address))
    ));
    let (jobs, _pending) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    crate::inventory_actions::handle(&mut state, &jobs, KeyCode::Esc);
    assert_eq!(state.mode, Mode::Terminal);
    assert_eq!(
        drawing_support::render(&state, panes::WIDTH, panes::HEIGHT)
            .1
            .bounds,
        before.bounds
    );
}

#[test]
fn toggling_sidebar_changes_only_available_geometry_not_tab_or_terminal_bindings() {
    let (mut state, _) = drawing_support::state();
    state.sidebar_visible = true;
    let original_layout = state.tab_layout().clone();
    let original_tab = state.tabs.active().id;
    let (_, visible) = drawing_support::render(&state, panes::WIDTH, panes::HEIGHT);
    let (jobs, mut pending) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    for visible_after in [false, true] {
        assert!(tabs::prefix(
            &mut state,
            &jobs,
            HubId::new(),
            KeyCode::Char(tabs::TOGGLE_SIDEBAR_KEY)
        ));
        let (_, current) = drawing_support::render(&state, panes::WIDTH, panes::HEIGHT);
        assert_eq!(state.sidebar_visible, visible_after);
        assert_eq!(*state.tab_layout(), original_layout);
        assert_eq!(state.tabs.active().id, original_tab);
        assert_eq!(current.bounds.y, visible.bounds.y);
        assert_eq!(current.bounds.height, visible.bounds.height);
        assert_eq!(
            current.bounds.width,
            if visible_after {
                visible.bounds.width
            } else {
                panes::WIDTH
            }
        );
    }
    assert!(pending.try_recv().is_err());
}

#[test]
fn narrow_sidebar_exposes_only_agents_and_hubs_and_keeps_terminal_state() {
    let (mut state, address) = drawing_support::state();
    let layout = state.tab_layout().clone();
    tabs::sidebar(&mut state);
    let (text, sidebar) = drawing_support::render(&state, NARROW_COLUMNS, panes::HEIGHT);
    assert!(!text.contains("Agent runs"));
    assert!(!text.contains("Terminals"));
    assert!(sidebar.panes.is_empty());
    state.mode = Mode::Terminal;
    let (_, terminal) = drawing_support::render(&state, NARROW_COLUMNS, panes::HEIGHT);
    assert_eq!(terminal.bounds.width, NARROW_COLUMNS);
    assert!(terminal.inventory.is_none());
    assert_eq!(*state.tab_layout(), layout);
    assert_eq!(state.focused_terminal(), Some(address));
}
