//! Durable agent-run selectors and queries.

use crate::arguments::{RunState, RunsArgs};
use crate::constants::{
    INVALID_LABEL_FILTER, RPC_TIMEOUT, RUN_NOT_FOUND, RUN_QUERIES_UNSUPPORTED, UNEXPECTED_REPLY,
};
use crate::outcome::Outcome;
use crate::reject::rejected;
use anyhow::{Result, bail};
use tokio::time::Instant;
use yolo_hub_agent_run_protocol::{
    AGENT_RUN_PROTOCOL_SCHEMA_VERSION, AgentRunFilter, AgentRunSelector, AgentRunState,
    GetAgentRunFields, GetAgentRunRequest, ListAgentRunsFields, ListAgentRunsRequest,
};
use yolo_hub_client::{ClientError, LocalClient};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

pub fn selector(hub: HubId, value: &str) -> Result<AgentRunSelector> {
    yolo_agent_catalog::run_selector(hub, value)
}

pub async fn runs(client: &mut LocalClient, args: RunsArgs) -> Result<Outcome> {
    let hub = client.hello().hub_id;
    if let Some(selector) = args.show.as_deref() {
        return show(client, hub, selector, args.output.json).await;
    }
    let request = ListAgentRunsRequest::new(ListAgentRunsFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        after: args.after.map(|id| yolo_agent_catalog::run_address(hub, id)).transpose()?,
        limit: args.limit,
        filter: filter(hub, &args)?,
    })?;
    let deadline = Instant::now() + RPC_TIMEOUT;
    let list = match client
        .call(hub, Command::ListAgentRuns { request }, deadline)
        .await
    {
        Ok(Reply::AgentRuns { list }) => list,
        Ok(_) => bail!(UNEXPECTED_REPLY),
        Err(ClientError::Remote(PublicError::Unsupported)) => {
            return Err(rejected(RUN_QUERIES_UNSUPPORTED))
        }
        Err(error) => return Err(error.into()),
    };
    let text = list
        .runs
        .iter()
        .map(yolo_agent_catalog::text)
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Outcome::success(
        args.output.json,
        serde_json::to_value(&list)?,
        text,
    ))
}

async fn show(client: &mut LocalClient, hub: HubId, selector: &str, json: bool) -> Result<Outcome> {
    let request = GetAgentRunRequest::new(GetAgentRunFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        selector: self::selector(hub, selector)?,
    })?;
    let deadline = Instant::now() + RPC_TIMEOUT;
    let run = match client
        .call(hub, Command::GetAgentRun { request }, deadline)
        .await
    {
        Ok(Reply::AgentRun { run }) => run,
        Ok(_) => bail!(UNEXPECTED_REPLY),
        Err(ClientError::Remote(PublicError::NotFound)) => return Err(rejected(RUN_NOT_FOUND)),
        Err(ClientError::Remote(PublicError::Unsupported)) => {
            return Err(rejected(RUN_QUERIES_UNSUPPORTED))
        }
        Err(error) => return Err(error.into()),
    };
    Ok(Outcome::success(
        json,
        serde_json::to_value(&run)?,
        yolo_agent_catalog::details(&run),
    ))
}

fn filter(hub: HubId, args: &RunsArgs) -> Result<Option<AgentRunFilter>> {
    Ok(if !args.state.is_empty() {
        Some(AgentRunFilter::State {
            is: args.state.iter().copied().map(state).collect(),
        })
    } else if let Some(id) = args.parent {
        Some(AgentRunFilter::Parent {
            address: yolo_agent_catalog::run_address(hub, id)?,
        })
    } else if let Some(provider) = &args.provider {
        Some(AgentRunFilter::Provider {
            id: provider.clone(),
        })
    } else if let Some(value) = &args.name {
        Some(AgentRunFilter::Name {
            value: value.clone(),
        })
    } else if let Some(label) = &args.label {
        let (key, value) = label
            .split_once('=')
            .ok_or_else(|| rejected(INVALID_LABEL_FILTER))?;
        Some(AgentRunFilter::Label {
            key: key.into(),
            value: value.into(),
        })
    } else {
        None
    })
}

fn state(value: RunState) -> AgentRunState {
    match value {
        RunState::Starting => AgentRunState::Starting,
        RunState::Launched => AgentRunState::Launched,
        RunState::Stopped => AgentRunState::Stopped,
    }
}
