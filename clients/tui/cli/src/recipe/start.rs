//! Sequential recipe execution over the ordinary `AgentStart` method.

use super::StartArgs;
use super::document::{RecipeAgent, RecipeDocument, read};
use crate::constants::*;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentStartFields, AgentStartRequest, AgentStartResult,
    AgentStopFields, AgentStopRequest,
};
use yolo_hub_agent_run_protocol::{AgentRunSelector, AgentRunSummary};
use yolo_hub_rpc_protocol::{Command as Rpc, Reply};
use yolo_hub_terminal_protocol::TerminalSize;
use yolo_hub_workspace_protocol::{
    GetWorkspaceFields, GetWorkspaceRequest, WORKSPACE_PROTOCOL_SCHEMA_VERSION, WorkspaceSelector,
};
use yolo_tui_app::Options;

pub(crate) async fn start(options: &Options, args: &StartArgs) -> Result<()> {
    let document = read(&args.file)?;
    let deadline = tokio::time::Instant::now() + CATALOG_TIMEOUT;
    let mut client = options.endpoint.connect(options.hub, deadline).await?;
    let hub = client.hello().hub_id;
    let cwd = match (&document.cwd, document.workspace) {
        (Some(cwd), _) => cwd.clone(),
        (None, Some(workspace)) => {
            let address = yolo_hub_workspace_protocol::address(hub, workspace)?;
            let request = GetWorkspaceRequest::new(GetWorkspaceFields {
                schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
                selector: WorkspaceSelector::Address(address),
            })?;
            match client
                .call(hub, Rpc::GetWorkspace { request }, deadline)
                .await?
            {
                Reply::Workspace { workspace } => workspace.root,
                _ => bail!(UNEXPECTED_RECIPE_REPLY),
            }
        }
        (None, None) => bail!(INVALID_RECIPE_CWD),
    };
    let parent = document
        .parent
        .as_deref()
        .map(|value| yolo_agent_catalog::run_selector(hub, value))
        .transpose()?;
    let mut started: Vec<AgentStartResult> = Vec::new();
    let mut failure = None;
    for index in super::compute(&document)? {
        let agent = &document.agents[index];
        if let Err(error) = super::wait::dependencies(&mut client, hub, agent, &started).await {
            failure = Some((agent, error));
            break;
        }
        let mut start =
            match start_one(&mut client, hub, &document, &cwd, parent.clone(), agent).await {
                Ok(start) => start,
                Err(error) => {
                    failure = Some((agent, error));
                    break;
                }
            };
        if let Err(error) =
            super::metadata::apply(&mut client, hub, &document, agent, &mut start).await
        {
            started.push(start);
            failure = Some((agent, error));
            break;
        }
        started.push(start);
    }
    let Some((agent, error)) = failure else {
        return print_started(&document, &started, args.json);
    };
    let stopped = if args.stop_on_failure || document.stop_on_failure {
        stop_started(&mut client, hub, &started).await
    } else {
        Vec::new()
    };
    report_failure(&document, &started, &stopped, agent, &error, args.json)?;
    Err(error)
}

async fn start_one(
    client: &mut yolo_hub_client::EndpointClient,
    hub: yolo_hub_identity::HubId,
    document: &RecipeDocument,
    cwd: &str,
    parent: Option<AgentRunSelector>,
    agent: &RecipeAgent,
) -> Result<AgentStartResult> {
    let request = AgentStartRequest::new(AgentStartFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        start_token: agent.start_token.unwrap_or_else(Uuid::new_v4),
        provider_id: agent.provider.clone(),
        name: agent.name.clone(),
        cwd: cwd.into(),
        size: TerminalSize::default(),
        workspace_id: document.workspace,
        parent,
    })?;
    let command = Rpc::AgentStart { request };
    let deadline = tokio::time::Instant::now() + command.operation_timeout(CATALOG_TIMEOUT);
    match client.call(hub, command, deadline).await? {
        Reply::AgentStart { start } => Ok(start),
        _ => bail!(UNEXPECTED_RECIPE_REPLY),
    }
}

async fn stop_started(
    client: &mut yolo_hub_client::EndpointClient,
    hub: yolo_hub_identity::HubId,
    started: &[AgentStartResult],
) -> Vec<Value> {
    let mut stopped = Vec::new();
    for start in started {
        let Some(run) = start.run.as_ref() else {
            continue;
        };
        stopped.push(stop_run(client, hub, run).await);
    }
    stopped
}

async fn stop_run(
    client: &mut yolo_hub_client::EndpointClient,
    hub: yolo_hub_identity::HubId,
    run: &AgentRunSummary,
) -> Value {
    let request = match AgentStopRequest::new(AgentStopFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        selector: AgentRunSelector::Address(run.address),
        force: false,
    }) {
        Ok(request) => request,
        Err(error) => return json!({"run": run.name, "error": error.to_string()}),
    };
    let command = Rpc::AgentStop { request };
    let deadline = tokio::time::Instant::now() + command.operation_timeout(CATALOG_TIMEOUT);
    match client.call(hub, command, deadline).await {
        Ok(Reply::AgentStop { stop }) => json!({"run": run.name, "outcome": stop.outcome}),
        Ok(_) => json!({"run": run.name, "error": "unexpected stop reply"}),
        Err(error) => json!({"run": run.name, "error": error.to_string()}),
    }
}

fn report_failure(
    document: &RecipeDocument,
    started: &[AgentStartResult],
    stopped: &[Value],
    agent: &RecipeAgent,
    error: &anyhow::Error,
    json: bool,
) -> Result<()> {
    if json {
        let payload = json!({
            "workspace": document.workspace,
            "started": started,
            "failed_agent": agent.name,
            "failure": error.to_string(),
            "stopped": stopped,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if !stopped.is_empty() {
        println!("stopped {} agent(s) after a failed start", stopped.len());
    }
    Ok(())
}

fn print_started(
    document: &RecipeDocument,
    started: &[AgentStartResult],
    json: bool,
) -> Result<()> {
    if json {
        let payload = json!({
            "workspace": document.workspace,
            "agents": started,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }
    for start in started {
        let name = start
            .run
            .as_ref()
            .map(|run| run.name.as_str())
            .unwrap_or("-");
        println!(
            "{name}\t{}\t{}",
            start.address.terminal_id, start.provider_id
        );
    }
    Ok(())
}
