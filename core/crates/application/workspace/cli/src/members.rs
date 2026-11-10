//! `workspace members`: the durable terminal and run bindings of one
//! workspace. Members are bound at creation; this command only reads them.

use crate::arguments::MembersArgs;
use crate::commands::{call, resolve, unexpected};
use anyhow::Result;
use serde_json::json;
use yolo_hub_agent_run_protocol::{
    AGENT_RUN_PROTOCOL_SCHEMA_VERSION, AgentRunSummary, ListAgentRunsFields,
    ListAgentRunsRequest,
};
use yolo_hub_client::LocalClient;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command as Rpc, Reply};
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_hub_workspace_protocol::WorkspaceAddress;

const PAGE: u16 = 128;
const MAX_PAGES: usize = 8;

pub async fn members(client: &mut LocalClient, hub: HubId, args: MembersArgs) -> Result<()> {
    let workspace = resolve(client, hub, &args.selector).await?;
    let destination = workspace.address.hub_id;
    let (terminals, terminals_truncated) = terminals(client, destination, workspace.address).await?;
    let (runs, runs_truncated) = runs(client, destination, workspace.address).await?;
    let truncated = terminals_truncated || runs_truncated;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "workspace": workspace,
                "terminals": terminals,
                "runs": runs,
                "truncated": truncated,
            }))?
        );
        return Ok(());
    }
    for terminal in &terminals {
        let fields = terminal.fields();
        println!(
            "terminal\t{}\t{}\tarchived={}",
            fields.address.terminal_id, fields.name, fields.archived
        );
    }
    for run in &runs {
        println!("run\t{}\t{}\t{:?}", run.address.run_id, run.name, run.state);
    }
    if truncated {
        println!("truncated");
    }
    Ok(())
}

async fn terminals(
    client: &mut LocalClient,
    hub: HubId,
    workspace: WorkspaceAddress,
) -> Result<(Vec<TerminalSummary>, bool)> {
    let mut after = None;
    let mut found = Vec::new();
    for _ in 0..MAX_PAGES {
        let reply = call(
            client,
            hub,
            Rpc::List {
                include_archived: true,
                after,
            },
        )
        .await?;
        let Reply::Terminals { terminals, next } = reply else {
            return Err(unexpected());
        };
        found.extend(
            terminals
                .into_iter()
                .filter(|terminal| terminal.fields().workspace == Some(workspace)),
        );
        let Some(cursor) = next else {
            return Ok((found, false));
        };
        after = Some(cursor);
    }
    Ok((found, true))
}

async fn runs(
    client: &mut LocalClient,
    hub: HubId,
    workspace: WorkspaceAddress,
) -> Result<(Vec<AgentRunSummary>, bool)> {
    let mut after = None;
    let mut found = Vec::new();
    for _ in 0..MAX_PAGES {
        let request = ListAgentRunsRequest::new(ListAgentRunsFields {
            schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
            after,
            limit: PAGE,
            filter: None,
        })?;
        let reply = call(client, hub, Rpc::ListAgentRuns { request }).await?;
        let Reply::AgentRuns { list } = reply else {
            return Err(unexpected());
        };
        found.extend(
            list.runs
                .into_iter()
                .filter(|run| run.workspace == Some(workspace)),
        );
        let Some(cursor) = list.next else {
            return Ok((found, false));
        };
        after = Some(cursor);
    }
    Ok((found, true))
}
