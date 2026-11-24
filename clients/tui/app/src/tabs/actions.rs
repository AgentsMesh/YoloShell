use crate::{Mode, State, network::Job};
use crossterm::event::KeyCode;
use tokio::sync::mpsc;
use yolo_hub_identity::HubId;
use yolo_presentation_layout::Axis;
use yolo_tui_inventory::View;

pub(crate) fn sidebar(state: &mut State) {
    super::terminal_picker::close(state);
    state.sidebar_visible = true;
    state.mode = Mode::Inventory;
    if state.inventory.view == View::Terminals {
        state.inventory.view = View::Tree;
    }
    state.presentation = Default::default();
}

pub(crate) fn prefix(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    hub: HubId,
    key: KeyCode,
) -> bool {
    match key {
        KeyCode::Char(super::PICKER_KEY) => super::terminal_picker::open(state),
        KeyCode::Char(super::TOGGLE_SIDEBAR_KEY) => {
            super::terminal_picker::close(state);
            state.sidebar_visible = !state.sidebar_visible;
            if !state.sidebar_visible {
                state.mode = Mode::Terminal;
            }
            state.presentation = Default::default();
        }
        KeyCode::Char(super::MANAGER_KEY) => {
            state.tab_manager = Some(yolo_tui_tab_manager::Manager::new(&state.tabs));
            state.presentation = Default::default();
        }
        KeyCode::Char(super::NEXT_KEY | super::PREVIOUS_KEY) => {
            let mut catalog = state.tab_snapshot();
            catalog.cycle(key == KeyCode::Char(super::NEXT_KEY));
            super::activate(state, catalog.active().id);
            crate::polling::acquire(state, jobs, false);
        }
        KeyCode::Char(super::SPLIT_RIGHT_KEY | '%') => split(state, jobs, hub, Axis::Horizontal),
        KeyCode::Char(super::SPLIT_DOWN_KEY | '"') => split(state, jobs, hub, Axis::Vertical),
        KeyCode::Char('x') => {
            super::close::pane(state);
        }
        KeyCode::Char(super::CLOSE_TAB_KEY) => {
            super::close::tab(state, state.tabs.active().id).expect(super::LAYOUT_INVARIANT);
        }
        KeyCode::Char(super::RESUME_KEY) => {
            let address = state.focused_terminal();
            super::terminal_picker::open(state);
            state.inventory.history = true;
            state.selected = address;
            state.reconcile_selection();
            state.status = if state.client_capability_enabled(
                yolo_builtin_client_contract::ClientCapability::TerminalLifecycle,
            ) {
                crate::constants::TERMINAL_RECOVERY_HINT
            } else {
                crate::constants::OBSERVER_HISTORY_HINT
            }
            .into();
        }
        _ => return false,
    }
    true
}

fn split(state: &mut State, jobs: &mpsc::Sender<Job>, hub: HubId, axis: Axis) {
    let origin = crate::shell_actions::destination(state, hub);
    let cwd = crate::shell_actions::directory(state, origin);
    super::creation::create(
        state,
        jobs,
        origin,
        cwd,
        None,
        super::creation::Placement::Split(axis),
    );
}
