use crate::{Mode, State, constants::*, form_actions::lifecycle, network::Job};
use crossterm::event::KeyCode;
use tokio::sync::mpsc;
use yolo_hub_terminal_protocol::TerminalAction;
use yolo_tui_inventory::View;

pub fn handle(state: &mut State, jobs: &mpsc::Sender<Job>, key: KeyCode) -> bool {
    if state.inventory.details.is_some() {
        match key {
            KeyCode::Esc => {
                state.inventory.details = None;
                return false;
            }
            KeyCode::Char('R') => {
                view_agent_terminal(state);
                return false;
            }
            KeyCode::Char('a') | KeyCode::Char('x') => {
                state.status = AGENT_HISTORY_DETAILS_LABEL.into();
                return false;
            }
            KeyCode::Char(c @ '1'..='9') => {
                crate::respond_actions::choose(
                    state,
                    jobs,
                    usize::from(c as u8 - b'1'),
                );
                return false;
            }
            KeyCode::Char('q') | KeyCode::Char(HISTORY_KEY) => {}
            _ => return false,
        }
    }
    match key {
        KeyCode::Char('q') => return true,
        KeyCode::Down | KeyCode::Char('j') => state.move_selection(true),
        KeyCode::Up | KeyCode::Char('k') => state.move_selection(false),
        KeyCode::Tab | KeyCode::BackTab => {
            if state.terminal_picker.is_some() {
                return false;
            }
            state.inventory.view = if state.inventory.view == View::Agents {
                View::Hubs
            } else {
                View::Agents
            };
            state.reconcile_selection();
        }
        KeyCode::Esc => {
            if state.terminal_picker.is_some() {
                crate::tabs::terminal_picker::close(state);
                return false;
            }
            if state.filter.is_empty()
                && state.inventory.scope.is_none()
                && !state.inventory.history
            {
                state.mode = Mode::Terminal;
                return false;
            }
            state.inventory.scope = None;
            state.inventory.history = false;
            state.filter.clear();
            state.reconcile_selection();
        }
        KeyCode::Char(HISTORY_KEY) => {
            state.inventory.toggle_history();
            state.reconcile_selection();
            state.status = if state.inventory.history {
                HISTORY_SCOPE_LABEL.into()
            } else {
                ACTIVE_SCOPE_LABEL.into()
            };
        }
        KeyCode::Char('/') => {
            state.mode = Mode::Search;
            crate::form_input::reset(state);
        }
        KeyCode::Char(crate::tabs::DETAILS_KEY) if state.inventory.view == View::Agents => {
            state.inventory.details = state.inventory.selected_agent;
        }
        KeyCode::Char('n') => {
            if state.inventory.view == View::Hubs {
                let Some(hub) = state.inventory.selected_hub else {
                    state.status = HUB_UNAVAILABLE_LABEL.into();
                    return false;
                };
                state.inventory.scope = Some(hub);
            }
            state.mode = Mode::Directory;
            state.entry = DEFAULT_DIRECTORY.into();
            crate::form_input::reset(state);
        }
        KeyCode::Char('r') => crate::catalog_events::invalidate(state, jobs),
        KeyCode::Char('R') | KeyCode::Char('x') | KeyCode::Char('a') => {
            // Agent state is not authority to terminate its containing shell.
            if state.inventory.view != View::Terminals {
                state.status = SESSION_CONTROL_ONLY_LABEL.into();
                return false;
            }
            if key == KeyCode::Char('R') {
                crate::terminal_recovery_actions::selected(state, jobs);
                return false;
            }
            let action = match key {
                KeyCode::Char('x') => TerminalAction::Terminate { force: false },
                _ => TerminalAction::Archive {},
            };
            lifecycle(state, jobs, action);
        }
        KeyCode::Enter => open(state, jobs),
        _ => {}
    }
    false
}

pub(crate) fn open(state: &mut State, jobs: &mpsc::Sender<Job>) {
    let address = match state.inventory.view {
        View::Hubs => {
            if let Some(hub) = state.inventory.selected_hub {
                state.inventory.scope = Some(hub);
                state.inventory.view = View::Agents;
                state.filter.clear();
                state.reconcile_selection();
            }
            return;
        }
        View::Agents => {
            let Some(target) = state.inventory.terminal_for_selected(&state.terminals) else {
                state.inventory.details = state.inventory.selected_agent;
                state.status = AGENT_HISTORY_DETAILS_LABEL.into();
                return;
            };
            target.address
        }
        View::Terminals => {
            let Some(address) = state.selected else {
                return;
            };
            address
        }
    };
    crate::tabs::terminal_picker::close(state);
    crate::tabs::open(state, jobs, address);
}

fn view_agent_terminal(state: &mut State) {
    let terminal = state
        .inventory
        .details
        .and_then(|address| state.inventory.agents.get(&address))
        .map(|agent| agent.fields().terminal.address);
    let Some(address) = terminal.filter(|address| state.terminals.contains_key(address)) else {
        state.status = AGENT_HISTORY_DETAILS_LABEL.into();
        return;
    };
    state.inventory.details = None;
    crate::tabs::terminal_picker::open(state);
    state.inventory.history = true;
    state.filter.clear();
    state.selected = Some(address);
    state.reconcile_selection();
    state.status = if state.client_capability_enabled(
        yolo_builtin_client_contract::ClientCapability::TerminalLifecycle,
    ) {
        TERMINAL_RECOVERY_HINT
    } else {
        OBSERVER_HISTORY_HINT
    }
    .into();
}
