use crate::constants::{EXECUTION_CHANGED, RPC_TIMEOUT, UNEXPECTED_REPLY};
use crate::reject::rejected;
use anyhow::{Result, bail};
use tokio::time::Instant;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_client::{ClientError, LocalClient};
use yolo_hub_identity::AgentTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

pub async fn snapshot(client: &mut LocalClient, target: AgentTarget) -> Result<AgentSnapshot> {
    optional(client, target)
        .await?
        .ok_or_else(|| rejected(EXECUTION_CHANGED))
}

pub async fn optional(
    client: &mut LocalClient,
    target: AgentTarget,
) -> Result<Option<AgentSnapshot>> {
    let deadline = Instant::now() + RPC_TIMEOUT;
    match client
        .call(
            target.address.hub_id,
            Command::GetAgent { target },
            deadline,
        )
        .await
    {
        Ok(Reply::Agent { agent }) => Ok(Some(agent)),
        Ok(_) => bail!(UNEXPECTED_REPLY),
        Err(ClientError::Remote(PublicError::NotFound)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}
