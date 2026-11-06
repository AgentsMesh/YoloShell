use anyhow::{Result, bail};
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{
    AGENT_RUN_PROTOCOL_SCHEMA_VERSION, AgentRunSelector, AgentRunSummary, GetAgentRunFields,
    GetAgentRunRequest, UpdateAgentRunFields, UpdateAgentRunRequest,
};
use yolo_hub_client::{ClientError, LocalClient};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

use crate::{
    constants::{INVALID_LABEL_FILTER, RPC_TIMEOUT, RUN_NOT_FOUND, RUN_QUERIES_UNSUPPORTED},
    reject::rejected,
};

pub async fn update_labels(
    client: &mut LocalClient,
    hub: HubId,
    selector: AgentRunSelector,
    labels: &[String],
    remove: &[String],
) -> Result<AgentRunSummary> {
    let request = GetAgentRunRequest::new(GetAgentRunFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        selector,
    })?;
    let deadline = Instant::now() + RPC_TIMEOUT;
    let run = match client.call(hub, Command::GetAgentRun { request }, deadline).await {
        Ok(Reply::AgentRun { run }) => run,
        Ok(_) => bail!(crate::constants::UNEXPECTED_REPLY),
        Err(ClientError::Remote(PublicError::NotFound)) => return Err(rejected(RUN_NOT_FOUND)),
        Err(ClientError::Remote(PublicError::Unsupported)) => {
            return Err(rejected(RUN_QUERIES_UNSUPPORTED))
        }
        Err(error) => return Err(error.into()),
    };
    let mut map = run.labels.clone();
    for value in labels {
        let (key, value) = value
            .split_once('=')
            .ok_or_else(|| rejected(INVALID_LABEL_FILTER))?;
        map.insert(key.into(), value.into());
    }
    for key in remove {
        map.remove(key);
    }
    let request = UpdateAgentRunRequest::new(UpdateAgentRunFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        address: run.address,
        expected_revision: run.revision,
        name: None,
        aliases: None,
        labels: Some(map),
    })?;
    let deadline = Instant::now() + RPC_TIMEOUT;
    match client
        .call(hub, Command::UpdateAgentRun { change: request }, deadline)
        .await
    {
        Ok(Reply::AgentRun { run }) => Ok(run),
        Ok(_) => bail!(crate::constants::UNEXPECTED_REPLY),
        Err(ClientError::Remote(PublicError::NotFound)) => Err(rejected(RUN_NOT_FOUND)),
        Err(ClientError::Remote(PublicError::Unsupported)) => Err(rejected(RUN_QUERIES_UNSUPPORTED)),
        Err(error) => Err(error.into()),
    }
}
