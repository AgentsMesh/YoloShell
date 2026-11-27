use crate::constants::*;
use anyhow::{Result, bail};
use tokio::time::Instant;
use yolo_hub_agent_run_protocol::{
    AGENT_RUN_PROTOCOL_SCHEMA_VERSION, AgentRunAddress, AgentRunSelector, AgentRunSummary,
    GetAgentRunFields, GetAgentRunRequest,
};
use yolo_hub_client::{ClientError, EndpointClient};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command as Rpc, Reply};
use yolo_hub_terminal_protocol::PublicError;

pub(super) async fn get_run(
    client: &mut EndpointClient,
    hub: HubId,
    address: AgentRunAddress,
) -> Result<AgentRunSummary> {
    let request = GetAgentRunRequest::new(GetAgentRunFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        selector: AgentRunSelector::Address(address),
    })?;
    let call_deadline = Instant::now() + CATALOG_TIMEOUT;
    match client
        .call(hub, Rpc::GetAgentRun { request }, call_deadline)
        .await?
    {
        Reply::AgentRun { run } => Ok(run),
        _ => bail!(UNEXPECTED_RECIPE_REPLY),
    }
}

/// A run whose terminal is gone or settled has exited even before the durable
/// run state catches up.
pub(super) async fn run_exited(
    client: &mut EndpointClient,
    run: &AgentRunSummary,
) -> Result<bool> {
    let Some(address) = run.terminal else {
        return Ok(false);
    };
    let command = Rpc::Get { address };
    let deadline = Instant::now() + CATALOG_TIMEOUT;
    match client.call(address.hub_id, command, deadline).await {
        Ok(Reply::Terminal { terminal }) => Ok(crate::lifecycle::settled(&terminal)),
        Ok(_) => bail!(UNEXPECTED_RECIPE_REPLY),
        Err(ClientError::Remote(PublicError::NotFound)) => Ok(true),
        Err(error) => Err(error.into()),
    }
}
