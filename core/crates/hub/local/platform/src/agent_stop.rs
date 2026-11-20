//! Stop an agent run by ending its terminal. The run transitions only after
//! the Host reports the terminal runtime is no longer live.

use crate::agent_run::{AgentRunService, terminal_observation};
use crate::errors::map_error;
use std::time::Duration;
use tokio::time::{Instant, sleep, timeout_at};
use yolo_client::{Client, ClientError};
use yolo_hub_agent_protocol::{AgentStopOutcome, AgentStopRequest, AgentStopResult};
use yolo_hub_agent_run_protocol::{AgentRunState, AgentRunSummary};
use yolo_hub_agent_run_store::TerminalObservation;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;
use yolo_terminal_model::TerminalLifecycleSelection;

const STOP_OBSERVATION_BUDGET: Duration = Duration::from_secs(2);
const STOP_OBSERVATION_INTERVAL: Duration = Duration::from_millis(50);

pub async fn agent_stop(
    service: &AgentRunService,
    client: &Client,
    request: &AgentStopRequest,
) -> Result<Reply, PublicError> {
    let (run, ended_runtime, observed_runtime) =
        service.stop_target(&request.fields().selector).await?;
    if run.state == AgentRunState::Stopped && ended_runtime.is_none() {
        return Ok(reply(run, AgentStopOutcome::AlreadyStopped));
    }
    let terminal = run.terminal.ok_or(PublicError::Conflict)?;
    let terminal_id = terminal.terminal_id.as_uuid();
    let view = match client.get_session(terminal_id).await {
        Ok(view) => view,
        Err(ClientError::Remote { code, .. }) if code == yolo_wire::error_code::NOT_FOUND => {
            if run.state == AgentRunState::Stopped {
                return Ok(reply(run, AgentStopOutcome::AlreadyStopped));
            }
            let refreshed = client
                .list_sessions_complete(true)
                .await
                .map_err(|_| PublicError::OutcomeUnknown)?;
            if refreshed
                .iter()
                .any(|view| view.manifest.terminal_id == terminal_id)
            {
                return Err(PublicError::OutcomeUnknown);
            }
            let run = service
                .mark_stopped_if_current(run.address, terminal, None, Some(observed_runtime))
                .await?;
            return Ok(reply(run, AgentStopOutcome::AlreadyExited));
        }
        Err(error) => return Err(map_error(error)),
    };
    if view.manifest.terminal_id != terminal_id {
        return Err(PublicError::TargetMismatch);
    }
    let mut expected_runtime = None;
    let outcome = match terminal_observation(&view) {
        Some(TerminalObservation::Live(epoch)) => {
            expected_runtime = Some(epoch);
            if ended_runtime == Some(epoch)
                || observed_runtime.is_some_and(|observed| observed != epoch)
            {
                return Err(PublicError::OutcomeUnknown);
            }
            let result = client
                .terminate_with_preconditions(
                    TerminalLifecycleSelection::at_runtime_epoch(terminal_id, epoch),
                    request.fields().force,
                )
                .await
                .map_err(map_error);
            if let Err(error) = result
                && !matches!(
                    error,
                    PublicError::OutcomeUnknown | PublicError::Unreachable
                )
            {
                return Err(error);
            }
            confirm_terminated(client, terminal_id, epoch).await?;
            AgentStopOutcome::Terminated
        }
        Some(TerminalObservation::Ended(epoch)) => {
            expected_runtime = Some(epoch);
            if observed_runtime.is_some_and(|observed| observed != epoch) {
                return Err(PublicError::OutcomeUnknown);
            }
            if run.state == AgentRunState::Stopped && ended_runtime == Some(epoch) {
                return Ok(reply(run, AgentStopOutcome::AlreadyStopped));
            }
            AgentStopOutcome::AlreadyExited
        }
        Some(TerminalObservation::Archived) => {
            if run.state == AgentRunState::Stopped {
                return Ok(reply(run, AgentStopOutcome::AlreadyStopped));
            }
            AgentStopOutcome::AlreadyExited
        }
        None => return Err(PublicError::Unreachable),
    };
    let run = service
        .mark_stopped_if_current(
            run.address,
            terminal,
            expected_runtime,
            Some(observed_runtime),
        )
        .await?;
    Ok(reply(run, outcome))
}

async fn confirm_terminated(
    client: &Client,
    terminal: uuid::Uuid,
    epoch: uuid::Uuid,
) -> Result<(), PublicError> {
    let deadline = Instant::now() + STOP_OBSERVATION_BUDGET;
    timeout_at(deadline, async {
        loop {
            if let Ok(view) = client.get_session(terminal).await {
                if view.manifest.terminal_id != terminal
                    || view
                        .runtime
                        .as_ref()
                        .is_some_and(|runtime| runtime.runtime_epoch != epoch)
                {
                    return Err(PublicError::OutcomeUnknown);
                }
                if matches!(
                    terminal_observation(&view),
                    Some(TerminalObservation::Ended(_) | TerminalObservation::Archived)
                ) {
                    return Ok(());
                }
            }
            sleep(STOP_OBSERVATION_INTERVAL).await;
        }
    })
    .await
    .map_err(|_| PublicError::OutcomeUnknown)?
}

fn reply(run: AgentRunSummary, outcome: AgentStopOutcome) -> Reply {
    Reply::AgentStop {
        stop: AgentStopResult { run, outcome },
    }
}
