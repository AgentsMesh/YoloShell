use crate::State;
use yolo_hub_client_channels::ControlError;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::PublicError;

const RECREATE_HINT: &str = "Recreation required · press R to confirm a new shell";

/// Control feedback belongs to a particular PTY incarnation. A delayed failure
/// from an older incarnation cannot change the state of its replacement.
pub(crate) fn obsolete(state: &State, command: &Command) -> bool {
    if let Command::Terminal { command } = command {
        return state
            .terminals
            .get(&command.selection.address)
            .is_some_and(|terminal| {
                terminal
                    .fields()
                    .runtime
                    .map(|runtime| runtime.target.runtime_epoch)
                    != command.selection.expected_runtime
            });
    }
    target(command).is_some_and(|target| {
        state
            .terminals
            .get(&target.address)
            .and_then(|terminal| terminal.fields().runtime)
            .is_some_and(|current| current.target != target)
    })
}

pub(crate) fn message(state: &State, command: &Command, error: &ControlError) -> String {
    if matches!(command, Command::Terminal { command }
        if matches!(command.action, yolo_hub_terminal_protocol::TerminalAction::Resume {}))
        && matches!(
            error,
            ControlError::Client(yolo_hub_client::ClientError::Remote(
                PublicError::RecreateRequired
            ))
        )
    {
        return RECREATE_HINT.into();
    }
    if matches!(
        error,
        ControlError::Client(yolo_hub_client::ClientError::Remote(PublicError::Conflict))
    ) && let Some(target) = target(command)
        && crate::observation_events::current(state, target)
        && let Some(hint) = crate::terminal_lifecycle::input_blocked_hint(state, target.address)
    {
        return hint.into();
    }
    crate::lease_events::failure_message(command, error)
}

fn target(command: &Command) -> Option<RuntimeTarget> {
    Some(match command {
        Command::ObserveLease { target } => *target,
        Command::Lease { request } => request.target,
        Command::Renew { grant } | Command::Release { grant } => grant.fields().target,
        Command::Attach { request } => request.target(),
        Command::Resize { request } => request.grant.fields().target,
        _ => return None,
    })
}
