use crate::caller;
use yolo_agent_catalog as catalog;
use crate::constants::*;
use crate::outcome::Outcome;
use crate::reject::rejected;
use crate::render;
use anyhow::Result;
use serde_json::json;
use std::time::Duration;
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentStartFields, AgentStartRequest, AgentStartResult,
    AgentStopFields, AgentStopOutcome, AgentStopRequest, AgentStopResult,
};
use yolo_hub_client::{ClientError, LocalClient};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{PublicError, TerminalSize, TerminalSummary};

/// Start a provider through the Hub's closed launch plan and wait a bounded
/// time for its first binding. Codex emits SessionStart only after the first
/// prompt, so a provider that has not bound yet is reported as awaiting rather
/// than as a failed launch.
pub async fn start(
    client: &mut LocalClient,
    provider: String,
    name: Option<String>,
    cwd: String,
    workspace: Option<Uuid>,
    current: bool,
    parent: Option<String>,
    timeout: u64,
    json: bool,
) -> Result<Outcome> {
    caller::hub(current, client.hello().hub_id.as_uuid())?;
    let workspace = caller::workspace(current, workspace)?;
    let parent = parent.or_else(caller::run);
    let started = start_agent(client, provider, name, cwd, workspace, parent).await?;
    let address = started.address;
    let deadline = Instant::now() + Duration::from_secs(timeout);
    let row = loop {
        if let Some(row) = catalog::agents(client, false)
            .await?
            .into_iter()
            .find(|row| row.snapshot.fields().terminal.address == address)
        {
            break row;
        }
        if Instant::now() >= deadline {
            let terminal = get_terminal(client, address).await?;
            return Ok(Outcome::success(
                json,
                awaiting_json(&terminal, &started),
                awaiting_text(&terminal, &started),
            ));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    };
    let terminal = row.terminal_name.as_deref();
    Ok(Outcome::success(
        json,
        render::agent_json(&row.snapshot, terminal)?,
        render::agent_text(&row.snapshot, terminal),
    ))
}

async fn start_agent(
    client: &mut LocalClient,
    provider: String,
    name: Option<String>,
    cwd: String,
    workspace: Option<Uuid>,
    parent: Option<String>,
) -> Result<AgentStartResult> {
    let hub = client.hello().hub_id;
    let parent = parent
        .map(|parent| crate::runs::selector(hub, &parent))
        .transpose()?;
    let request = AgentStartRequest::new(AgentStartFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        start_token: Uuid::new_v4(),
        provider_id: provider,
        name,
        cwd,
        size: TerminalSize::default(),
        workspace_id: workspace,
        parent,
    })?;
    let destination = client.hello().hub_id;
    let command = Command::AgentStart { request };
    let deadline = Instant::now() + command.operation_timeout(RPC_TIMEOUT);
    match client.call(destination, command, deadline).await {
        Ok(Reply::AgentStart { start }) => Ok(start),
        Ok(_) => Err(rejected(UNEXPECTED_REPLY)),
        Err(ClientError::Remote(PublicError::Unsupported)) => {
            Err(rejected(AGENT_START_UNSUPPORTED))
        }
        Err(error) => Err(error.into()),
    }
}

async fn get_terminal(
    client: &mut LocalClient,
    address: TerminalAddress,
) -> Result<TerminalSummary> {
    let deadline = Instant::now() + RPC_TIMEOUT;
    match client
        .call(address.hub_id, Command::Get { address }, deadline)
        .await?
    {
        Reply::Terminal { terminal } => Ok(terminal),
        _ => Err(rejected(UNEXPECTED_REPLY)),
    }
}

fn awaiting_json(terminal: &TerminalSummary, started: &AgentStartResult) -> serde_json::Value {
    let fields = terminal.fields();
    json!({
        "binding": "awaiting",
        "provider": started.provider_id,
        "terminal": {
            "address": fields.address,
            "name": fields.name,
        },
        "agent": null,
        "run": started.run,
    })
}

fn awaiting_text(terminal: &TerminalSummary, started: &AgentStartResult) -> String {
    let mut text = format!(
        "terminal: {}\nbinding: awaiting; prompt the agent to observe it",
        terminal.fields().name
    );
    if let Some(run) = &started.run {
        text.push_str(&format!("\nrun: {} ({})", run.name, run.address.run_id));
    }
    text
}

/// Stop an agent run by ending its terminal. The Hub records the run as
/// stopped only after it confirms the terminal runtime is no longer live.
pub async fn stop(
    client: &mut LocalClient,
    run: String,
    force: bool,
    json: bool,
) -> Result<Outcome> {
    let hub = client.hello().hub_id;
    let selector = crate::runs::selector(hub, &run)?;
    let request = AgentStopRequest::new(AgentStopFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        selector,
        force,
    })?;
    let deadline = Instant::now() + CONTROL_TIMEOUT;
    match client
        .call(hub, Command::AgentStop { request }, deadline)
        .await?
    {
        Reply::AgentStop { stop } => Ok(Outcome::success(
            json,
            serde_json::to_value(&stop)?,
            stop_text(&stop),
        )),
        _ => Err(rejected(UNEXPECTED_REPLY)),
    }
}

fn stop_text(result: &AgentStopResult) -> String {
    let outcome = match result.outcome {
        AgentStopOutcome::Terminated => "terminated",
        AgentStopOutcome::AlreadyExited => "already_exited",
        AgentStopOutcome::AlreadyStopped => "already_stopped",
    };
    format!("{} {outcome}", result.run.name)
}
