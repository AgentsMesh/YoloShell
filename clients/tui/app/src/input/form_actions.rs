use crate::{Mode, State, events::enqueue, network::Job};
use crossterm::event::KeyCode;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{TerminalAction, TerminalCommand, TerminalSelection};

pub(crate) fn entry(state: &mut State, jobs: &mpsc::Sender<Job>, hub: HubId, key: KeyCode) {
    match key {
        KeyCode::Esc => state.mode = Mode::Inventory,
        KeyCode::Enter => {
            if state.mode == Mode::Directory {
                let origin = crate::shell_actions::destination(state, hub);
                if !crate::shell_actions::create(state, jobs, origin, state.entry.clone(), None) {
                    let message = state.status.clone();
                    crate::form_input::fail(state, message);
                }
                return;
            }
            state.mode = Mode::Inventory;
        }
        _ => {}
    }
}

pub(crate) fn lifecycle(state: &mut State, jobs: &mpsc::Sender<Job>, action: TerminalAction) {
    if !crate::client_capabilities::require(
        state,
        ClientCapability::TerminalLifecycle,
        crate::client_capabilities::TERMINAL_LIFECYCLE_UNAVAILABLE,
    ) {
        return;
    }
    let Some(session) = state
        .selected
        .and_then(|address| state.terminals.get(&address))
    else {
        return;
    };
    let fields = session.fields();
    let origin = fields.address.hub_id;
    let command = TerminalCommand {
        request_id: Uuid::new_v4(),
        selection: TerminalSelection {
            address: fields.address,
            expected_runtime: fields.runtime.map(|runtime| runtime.target.runtime_epoch),
        },
        action,
    };
    state.status.clear();
    match action {
        TerminalAction::Terminate { .. } => {
            state.mode = Mode::ConfirmTerminate;
            state.confirmation = Some(command);
        }
        TerminalAction::Archive {} => {
            state.mode = Mode::ConfirmArchive;
            state.confirmation = Some(command);
        }
        TerminalAction::Recreate {} => {
            state.mode = Mode::ConfirmRecreate;
            state.confirmation = Some(command);
        }
        _ => {
            enqueue(
                state,
                jobs,
                Job::Call {
                    generation: None,
                    origin,
                    command: Command::Terminal { command },
                },
            );
        }
    }
}
