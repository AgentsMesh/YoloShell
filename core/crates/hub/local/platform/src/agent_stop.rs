//! Stop an agent run by ending its terminal. The run transitions only after
//! the Host reports the terminal runtime is no longer live.

use crate::agent_run::AgentRunService;
use crate::errors::map_error;
use yolo_client::{Client, ClientError};
use yolo_hub_agent_protocol::{AgentStopOutcome, AgentStopRequest, AgentStopResult};
use yolo_hub_agent_run_protocol::{AgentRunState, AgentRunSummary};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;
use yolo_terminal_model::{TerminalLifecycleSelection, TerminalRuntimeStatus};

pub async fn agent_stop(
    service: &AgentRunService,
    client: &Client,
    request: &AgentStopRequest,
) -> Result<Reply, PublicError> {
    let run = service.resolve(&request.fields().selector).await?;
    if run.state == AgentRunState::Stopped {
        return Ok(reply(run, AgentStopOutcome::AlreadyStopped));
    }
    let terminal = run.terminal.ok_or(PublicError::Conflict)?;
    let terminal_id = terminal.terminal_id.as_uuid();
    let view = match client.get_session(terminal_id).await {
        Ok(view) => view,
        Err(ClientError::Remote { code, .. }) if code == yolo_wire::error_code::NOT_FOUND => {
            let run = service.mark_stopped(run.address).await?;
            return Ok(reply(run, AgentStopOutcome::AlreadyExited));
        }
        Err(error) => return Err(map_error(error)),
    };
    let outcome = if view.runtime.as_ref().is_some_and(live) {
        client
            .terminate_with_preconditions(
                TerminalLifecycleSelection::unconditional(terminal_id),
                request.fields().force,
            )
            .await
            .map_err(map_error)?;
        match client.get_session(terminal_id).await {
            Ok(after) if after.runtime.as_ref().is_some_and(live) => {
                return Err(PublicError::OutcomeUnknown);
            }
            Ok(_) => {}
            Err(ClientError::Remote { code, .. }) if code == yolo_wire::error_code::NOT_FOUND => {}
            Err(_) => return Err(PublicError::OutcomeUnknown),
        }
        AgentStopOutcome::Terminated
    } else {
        AgentStopOutcome::AlreadyExited
    };
    let run = service.mark_stopped(run.address).await?;
    Ok(reply(run, outcome))
}

fn live(runtime: &yolo_terminal_model::RuntimeView) -> bool {
    matches!(
        runtime.status,
        TerminalRuntimeStatus::Starting
            | TerminalRuntimeStatus::Running
            | TerminalRuntimeStatus::Finalizing
            | TerminalRuntimeStatus::OutputBlocked
    )
}

fn reply(run: AgentRunSummary, outcome: AgentStopOutcome) -> Reply {
    Reply::AgentStop {
        stop: AgentStopResult { run, outcome },
    }
}
