use crate::polling::acquire;
use crate::{State, constants::*, events::enqueue, network::Job};
use crossterm::event::KeyCode;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{InputFields, InputRequest};

pub(crate) fn prefix(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    hub: yolo_hub_identity::HubId,
    key: KeyCode,
) -> bool {
    state.pointer = crate::pointer::Pointer::default();
    if key == KeyCode::Char(crate::chat::TOGGLE_KEY) {
        crate::chat::toggle(state);
        return false;
    }
    if key == KeyCode::Char(crate::finalization_persistence::RECOVER_STORAGE_KEY) {
        let now = std::time::Instant::now();
        crate::finalization_persistence::request_recovery(state, now);
        crate::creation_confirmation::flush(state, jobs, now);
        crate::finalization_actions::flush(state, jobs, now);
        return false;
    }
    if key == KeyCode::Char(crate::selection_state::COPY_KEY) {
        crate::selection_actions::copy(state);
        return false;
    }
    crate::selection_actions::cancel(state);
    if crate::tabs::prefix(state, jobs, hub, key) {
        return false;
    }
    if key == KeyCode::Char(crate::help_constants::HELP_KEY) {
        crate::help::open(state);
        return false;
    }
    if key == KeyCode::Char(crate::help_constants::WORKSPACE_KEY) {
        crate::workspace_ui::open(state, jobs, hub);
        return false;
    }
    let previous_pane = state.tab_layout().focused();
    if let Some(result) = crate::pane_actions::apply(state, key) {
        if let Err(error) = result {
            state.status = error.to_string();
        } else if state.tab_layout().focused() != previous_pane {
            state.mode = crate::Mode::Terminal;
            acquire(state, jobs, false);
        }
        return false;
    }
    let result = match key {
        KeyCode::Char(crate::help_constants::SESSIONS_KEY) => {
            crate::tabs::sidebar(state);
            Ok(())
        }
        KeyCode::Char(crate::help_constants::DETACH_KEY | crate::help_constants::QUIT_KEY) => {
            return true;
        }
        KeyCode::Char('o') => {
            let panes = state.tab_layout().panes();
            let index = panes
                .iter()
                .position(|pane| pane.id == state.tab_layout().focused())
                .unwrap_or_default();
            let next = panes[(index + 1) % panes.len()].id;
            state.tab_layout_mut().focus(next).map(|_| {
                state.mode = crate::Mode::Terminal;
                acquire(state, jobs, false);
            })
        }
        KeyCode::Char(crate::help_constants::NEW_SHELL_KEY) => {
            crate::shell_actions::current_directory(state, jobs, hub);
            Ok(())
        }
        KeyCode::Char(crate::help_constants::ACQUIRE_KEY) => {
            crate::polling::resume_input(state, jobs, false);
            Ok(())
        }
        KeyCode::Char(crate::help_constants::TAKEOVER_KEY) => {
            crate::polling::resume_input(state, jobs, true);
            Ok(())
        }
        KeyCode::Char('r') => {
            if let Some(address) = state.focused_terminal() {
                if let Some(runtime) = state
                    .terminals
                    .get(&address)
                    .and_then(|terminal| terminal.fields().runtime)
                {
                    state.control.observe(runtime.target);
                }
                crate::terminal_input::clear_pregrant_for(state, address);
                if let Some(writer) = state.retire_writer(address) {
                    enqueue(
                        state,
                        jobs,
                        Job::Call {
                            generation: None,
                            origin: address.hub_id,
                            command: Command::Release {
                                grant: writer.grant,
                            },
                        },
                    );
                }
            }
            Ok(())
        }
        _ => Ok(()),
    };
    if let Err(error) = result {
        state.status = error.to_string();
    }
    false
}

include!("terminal_delivery.rs");
