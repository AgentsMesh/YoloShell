use crate::State;
use yolo_hub_agent_protocol::{AgentAction, CommandOutcome};
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};

const SENT: &str = "Answer sent";
const REJECTED: &str = "Answer rejected; refresh the Agent";
const UNKNOWN: &str = "Answer outcome unknown; refresh the Agent";
const NOT_SENT: &str = "Answer not sent";

pub(crate) fn finished(
    state: &mut State,
    request: &ControlRequest,
    result: &Result<Reply, ControlError>,
) -> Option<bool> {
    let (target, request_id) = respond_target(request)?;
    state.respond_pending.remove(&target);
    let next = match result {
        Ok(Reply::AgentControl { receipt })
            if receipt.request_id() == request_id && receipt.target() == target =>
        {
            match receipt.outcome() {
                CommandOutcome::Accepted {} => "Answer accepted; awaiting result",
                CommandOutcome::Delivered {} | CommandOutcome::Confirmed {} => SENT,
                CommandOutcome::Failed { .. } => REJECTED,
                CommandOutcome::Unknown {} => UNKNOWN,
            }
        }
        Ok(_) => UNKNOWN,
        Err(
            ControlError::NotConnected
            | ControlError::StaleGeneration
            | ControlError::Invalidated
            | ControlError::OperationLimit
            | ControlError::QueryLimit,
        ) => NOT_SENT,
        Err(_) => UNKNOWN,
    };
    if selected(state, target) {
        state.status = next.into();
    }
    Some(true)
}

pub(crate) fn not_sent(state: &mut State, request: &ControlRequest) -> bool {
    let Some((target, _)) = respond_target(request) else {
        return false;
    };
    state.respond_pending.remove(&target);
    if selected(state, target) {
        state.status = NOT_SENT.into();
    }
    true
}

fn respond_target(
    request: &ControlRequest,
) -> Option<(yolo_hub_identity::AgentTarget, uuid::Uuid)> {
    let Command::AgentControl { request, .. } = &request.command else {
        return None;
    };
    if !matches!(request.fields().action, AgentAction::Respond { .. }) {
        return None;
    }
    Some((request.fields().target, request.fields().request_id))
}

fn selected(state: &State, target: yolo_hub_identity::AgentTarget) -> bool {
    state
        .inventory
        .selected_agent
        .and_then(|address| state.inventory.agents.get(&address))
        .is_some_and(|agent| agent.cursor().target() == target)
}
