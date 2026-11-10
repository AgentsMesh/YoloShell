use crate::{State, interrupt_state::Phase};
use yolo_hub_agent_protocol::{CommandOutcome, FailureCode};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

pub(crate) fn finished(
    state: &mut State,
    request: &ControlRequest,
    result: &Result<Reply, ControlError>,
) -> Option<bool> {
    if !matches!(&request.command, Command::AgentControl { request, .. }
        if matches!(request.fields().action, yolo_hub_agent_protocol::AgentAction::Interrupt {}))
    {
        return None;
    }
    let Some(entry) = state.interrupts.correlated(request) else {
        return Some(false);
    };
    let next = match result {
        Ok(Reply::AgentControl { receipt })
            if receipt.request_id() == entry.request.fields().request_id
                && receipt.target() == entry.request.fields().target =>
        {
            match receipt.outcome() {
                CommandOutcome::Accepted {} => Phase::Accepted,
                CommandOutcome::Delivered {} | CommandOutcome::Confirmed {} => Phase::Sent,
                CommandOutcome::Failed { code } => Phase::Rejected(code),
                CommandOutcome::Unknown {} => Phase::Unknown,
            }
        }
        Ok(_) => Phase::Unknown,
        Err(
            ControlError::NotConnected
            | ControlError::StaleGeneration
            | ControlError::Invalidated
            | ControlError::OperationLimit
            | ControlError::QueryLimit,
        ) => Phase::NotSent,
        Err(ControlError::QueryTaskFailed) => Phase::Unknown,
        Err(ControlError::Client(error)) => failure(error),
    };
    if !(entry.phase == Phase::Unknown && next == Phase::Accepted) {
        entry.phase = next;
    }
    let target = entry.request.fields().target;
    let label = entry.phase.label();
    if selected(state, target) {
        state.status = label.into();
    }
    Some(true)
}

pub(crate) fn not_sent(state: &mut State, request: &ControlRequest) -> bool {
    if !matches!(&request.command, Command::AgentControl { request, .. }
        if matches!(request.fields().action, yolo_hub_agent_protocol::AgentAction::Interrupt {}))
    {
        return false;
    }
    let Some(entry) = state.interrupts.correlated(request) else {
        return true;
    };
    entry.phase = Phase::NotSent;
    let target = entry.request.fields().target;
    if selected(state, target) {
        state.status = crate::interrupt_constants::NOT_SENT.into();
    }
    true
}

fn selected(state: &State, target: yolo_hub_identity::AgentTarget) -> bool {
    state
        .inventory
        .selected_agent
        .and_then(|address| state.inventory.agents.get(&address))
        .is_some_and(|agent| agent.cursor().target() == target)
}

fn failure(error: &ClientError) -> Phase {
    if error.outcome_is_unknown() {
        return Phase::Unknown;
    }
    match error {
        ClientError::Validation(_) | ClientError::Unsupported | ClientError::WrongLocalUser => {
            Phase::NotSent
        }
        ClientError::Remote(PublicError::Unauthorized) => {
            Phase::Rejected(FailureCode::PermissionDenied)
        }
        ClientError::Remote(PublicError::Unsupported) => Phase::Rejected(FailureCode::Unsupported),
        ClientError::Remote(PublicError::Conflict | PublicError::TargetMismatch) => {
            Phase::Rejected(FailureCode::PreconditionsChanged)
        }
        ClientError::Remote(_) => Phase::Rejected(FailureCode::AdapterRejected),
        _ => Phase::Unknown,
    }
}
