use crate::{State, form_actions, network::Job};
use tokio::sync::mpsc;
use yolo_hub_terminal_protocol::{TerminalAction, TerminalRecoveryHint};

pub(crate) const CONFIRM_RECREATE: &str =
    "New shell; agent will not resume. Enter confirms · Esc cancels";
const RECOVERING: &str = "Terminal recovery is already in progress";
const UNAVAILABLE: &str = "Recovery is unavailable for this terminal; Ctrl-b c creates a new shell";

pub(crate) fn selected(state: &mut State, jobs: &mpsc::Sender<Job>) {
    let Some(terminal) = state
        .selected
        .and_then(|address| state.terminals.get(&address))
    else {
        return;
    };
    match terminal
        .fields()
        .recovery
        .unwrap_or(TerminalRecoveryHint::Unavailable)
    {
        TerminalRecoveryHint::Ready | TerminalRecoveryHint::Resume => {
            form_actions::lifecycle(state, jobs, TerminalAction::Resume {});
        }
        TerminalRecoveryHint::Recreate => {
            form_actions::lifecycle(state, jobs, TerminalAction::Recreate {});
        }
        TerminalRecoveryHint::Recovering => state.status = RECOVERING.into(),
        TerminalRecoveryHint::Unavailable => state.status = UNAVAILABLE.into(),
    }
}
