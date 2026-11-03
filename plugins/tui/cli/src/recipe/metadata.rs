//! Post-start metadata: merged labels are applied first, then the optional prompt.

use super::document::{RecipeAgent, RecipeDocument};
use crate::constants::*;
use anyhow::{Result, bail};
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_agent_protocol::AgentStartResult;
use yolo_hub_agent_run_protocol::{
    AGENT_RUN_PROTOCOL_SCHEMA_VERSION, UpdateAgentRunFields, UpdateAgentRunRequest,
};
use yolo_hub_client::EndpointClient;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command as Rpc, Reply};

pub(crate) fn merged(
    document: &BTreeMap<String, String>,
    agent: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut labels = document.clone();
    labels.extend(agent.clone());
    labels
}

pub(super) async fn apply(
    client: &mut EndpointClient,
    hub: HubId,
    document: &RecipeDocument,
    agent: &RecipeAgent,
    start: &mut AgentStartResult,
) -> Result<()> {
    let labels = merged(&document.labels, &agent.labels);
    apply_labels(client, hub, start, &labels).await?;
    super::prompt::apply(client, hub, agent, start).await
}

async fn apply_labels(
    client: &mut EndpointClient,
    hub: HubId,
    start: &mut AgentStartResult,
    labels: &BTreeMap<String, String>,
) -> Result<()> {
    if labels.is_empty() {
        return Ok(());
    }
    let Some(run) = start.run.as_ref() else {
        bail!(RECIPE_LABELS_REQUIRE_RUN);
    };
    let request = UpdateAgentRunRequest::new(UpdateAgentRunFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        address: run.address,
        expected_revision: run.revision,
        name: None,
        aliases: None,
        labels: Some(labels.clone()),
    })?;
    let command = Rpc::UpdateAgentRun { change: request };
    let deadline = tokio::time::Instant::now() + command.operation_timeout(CATALOG_TIMEOUT);
    match client.call(hub, command, deadline).await? {
        Reply::AgentRun { run } => {
            start.run = Some(run);
            Ok(())
        }
        _ => bail!(UNEXPECTED_RECIPE_REPLY),
    }
}
