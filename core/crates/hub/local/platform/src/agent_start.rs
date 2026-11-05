use crate::{
    agent_run::{AgentRunService, run_name},
    caller_context,
    creation_request::runtime_agent_start_request,
    launch::{address, create_finalized},
};
use yolo_agent_provider_contract::LaunchPolicies;
use yolo_client::Client;
use yolo_hub_agent_protocol::{AgentStartBinding, AgentStartRequest, AgentStartResult};
use yolo_hub_agent_run_protocol::AgentRunSummary;
use yolo_hub_identity::{HubId, TerminalAddress, WorkspaceAddress};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

/// Start a known provider in a new terminal through the Hub's closed launch
/// plan. The result is Awaiting: the terminal and launch input exist, but the
/// provider may not have emitted a binding yet (Codex waits for the first
/// prompt).
pub async fn agent_start(
    hub: HubId,
    client: &Client,
    launches: Option<&LaunchPolicies>,
    runs: Option<&AgentRunService>,
    workspaces: Option<&crate::workspace::WorkspaceService>,
    request: &AgentStartRequest,
) -> Result<Reply, PublicError> {
    let fields = request.fields();
    let context = crate::caller_context::caller_context(
        hub,
        workspaces,
        fields.workspace_id,
        runs.is_some().then_some(fields.start_token),
    )
    .await?;
    let workspace = caller_context::workspace_address(hub, context);
    let _reservation = match (workspaces, workspace) {
        (Some(service), Some(address)) => Some(service.reserve_creation(address).await?),
        _ => None,
    };
    let command = launches
        .and_then(|policies| policies.command(&fields.provider_id))
        .ok_or(PublicError::Unsupported)?;
    let parent = match (fields.parent.as_ref(), runs) {
        (None, _) => None,
        (Some(selector), Some(service)) => {
            let parent = service.resolve(selector).await?.address;
            if parent.hub_id != hub {
                return Err(PublicError::OriginMismatch);
            }
            Some(parent)
        }
        (Some(_), None) => return Err(PublicError::Unsupported),
    };
    let mut reserved = match runs {
        None => None,
        Some(service) => match service.resume(fields.start_token).await? {
            Some(existing) => {
                if existing.provider_id != fields.provider_id
                    || existing.workspace != workspace
                    || existing.parent != parent
                    || fields
                        .name
                        .as_ref()
                        .is_some_and(|name| !existing.name.eq_ignore_ascii_case(name))
                {
                    return Err(PublicError::Conflict);
                }
                if existing.state == yolo_hub_agent_run_protocol::AgentRunState::Stopped {
                    return Err(PublicError::Conflict);
                }
                if existing.state == yolo_hub_agent_run_protocol::AgentRunState::Launched {
                    let terminal = existing.terminal.ok_or(PublicError::Internal)?;
                    bind(workspaces, workspace, terminal, Some(existing.clone())).await?;
                    return Ok(reply(fields, Some(existing.clone()), terminal));
                }
                service.rejoin_starting(existing.address).await?;
                Some(existing)
            }
            None => Some(
                service
                    .reserve(
                        fields.start_token,
                        fields
                            .name
                            .clone()
                            .unwrap_or_else(|| run_name(&fields.provider_id, fields.start_token)),
                        fields.provider_id.clone(),
                        workspace,
                        parent,
                        fields.start_token,
                    )
                    .await?,
            ),
        },
    };
    let created = match create_finalized(
        client,
        &runtime_agent_start_request(request, command, context),
    )
    .await
    {
        Ok(created) => created,
        Err(error) => {
            if definitely_absent(error) {
                if let (Some(service), Some(run)) = (runs, reserved.as_ref()) {
                    let _ = service.cancel_starting(run.address).await;
                }
            }
            return Err(error);
        }
    };
    let terminal = address(hub, created.terminal_id)?;
    if let (Some(service), Some(run)) = (runs, reserved.take()) {
        let launched = service.mark_launched(run.address, terminal).await?;
        bind(workspaces, workspace, terminal, Some(launched.clone())).await?;
        reserved = Some(launched);
    } else {
        bind(workspaces, workspace, terminal, None).await?;
    }
    Ok(reply(fields, reserved, terminal))
}

async fn bind(
    workspaces: Option<&crate::workspace::WorkspaceService>,
    workspace: Option<WorkspaceAddress>,
    terminal: TerminalAddress,
    run: Option<AgentRunSummary>,
) -> Result<(), PublicError> {
    let (Some(service), Some(workspace)) = (workspaces, workspace) else {
        return Ok(());
    };
    service
        .bind_terminal(workspace, terminal.terminal_id.as_uuid())
        .await?;
    if let Some(run) = run {
        service
            .bind_run(workspace, run.address.run_id.as_uuid())
            .await?;
    }
    Ok(())
}

/// Unknown/retryable creation failures may have produced a terminal; only a
/// definite rejection proves that the reservation can be cancelled.
fn definitely_absent(error: PublicError) -> bool {
    matches!(
        error,
        PublicError::InvalidDirectory
            | PublicError::InvalidRequest
            | PublicError::InvalidMetadata
            | PublicError::InvalidPayload
            | PublicError::Unsupported
            | PublicError::RecreateRequired
            | PublicError::LimitExceeded
    )
}

fn reply(
    fields: &yolo_hub_agent_protocol::AgentStartFields,
    run: Option<yolo_hub_agent_run_protocol::AgentRunSummary>,
    terminal: yolo_hub_identity::TerminalAddress,
) -> Reply {
    Reply::AgentStart {
        start: AgentStartResult {
            address: terminal,
            provider_id: fields.provider_id.clone(),
            binding: AgentStartBinding::Awaiting,
            run,
        },
    }
}
