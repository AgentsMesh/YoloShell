use crate::arguments::{
    Arguments, Command, CreateArgs, GetArgs, UpdateArgs, WorkspaceCommand,
};
use crate::connection::{self, Connection};
use crate::constants::*;
use anyhow::{Result, anyhow};
use std::collections::BTreeMap;
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_client::LocalClient;
use yolo_hub_identity::{HubId, WorkspaceId};
use yolo_hub_rpc_protocol::{Command as Rpc, Reply};
use yolo_hub_workspace_protocol::{
    CreateWorkspaceFields, CreateWorkspaceRequest, ListWorkspacesResult,
    MembershipChange, UpdateWorkspaceFields, UpdateWorkspaceRequest,
    WORKSPACE_PROTOCOL_SCHEMA_VERSION, WorkspaceAddress, WorkspaceSummary,
};

pub(crate) use crate::selector::resolve;

pub async fn run(arguments: Arguments) -> Result<()> {
    match arguments.command {
        Command::Workspace(options) => run_workspace(options).await,
        Command::Worktree(options) => run_worktree(options).await,
    }
}

pub async fn run_workspace(options: crate::arguments::Options) -> Result<()> {
    let mut client = connection::connect(&Connection {
        socket: options.socket,
        hub: options.hub,
        local_profile: options.local_profile,
    })
    .await?;
    let hub = client.hello().hub_id;
    match options.command {
        WorkspaceCommand::Create(args) => create(&mut client, hub, args).await,
        WorkspaceCommand::Update(args) => update(&mut client, hub, args).await,
        WorkspaceCommand::Get(args) => get(&mut client, hub, args).await,
        WorkspaceCommand::List(args) => crate::listing::list(&mut client, hub, args).await,
        WorkspaceCommand::Members(args) => crate::members::members(&mut client, hub, args).await,
    }
}

pub async fn run_worktree(options: crate::arguments::WorktreeOptions) -> Result<()> {
    crate::worktree::run(options).await
}

async fn create(client: &mut LocalClient, hub: HubId, args: CreateArgs) -> Result<()> {
    let request = CreateWorkspaceRequest::new(CreateWorkspaceFields {
        schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        workspace_id: Uuid::new_v4(),
        name: args.name,
        root: args.root,
        labels: labels(&args.labels)?,
    })?;
    let workspace = match call(client, hub, Rpc::CreateWorkspace { request }).await? {
        Reply::Workspace { workspace } => workspace,
        _ => return Err(unexpected()),
    };
    print(&workspace, args.json)
}

async fn update(client: &mut LocalClient, hub: HubId, args: UpdateArgs) -> Result<()> {
    let current = resolve(client, hub, &args.selector).await?;
    let members = membership(&args)?;
    let labels = if args.labels.is_empty() {
        None
    } else {
        Some(labels(&args.labels)?)
    };
    let request = UpdateWorkspaceRequest::new(UpdateWorkspaceFields {
        schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        address: current.address,
        expected_revision: args.expected_revision.unwrap_or(current.revision),
        name: args.name,
        labels,
        members,
    })?;
    let destination = current.address.hub_id;
    let workspace = match call(
        client,
        destination,
        Rpc::UpdateWorkspace { change: request },
    )
    .await?
    {
        Reply::Workspace { workspace } => workspace,
        _ => return Err(unexpected()),
    };
    print(&workspace, args.json)
}

async fn get(client: &mut LocalClient, hub: HubId, args: GetArgs) -> Result<()> {
    let workspace = resolve(client, hub, &args.selector).await?;
    print(&workspace, args.json)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceCursor {
    Address(WorkspaceAddress),
    Id(Uuid),
}

impl std::str::FromStr for WorkspaceCursor {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Ok(id) = Uuid::parse_str(value) {
            return Ok(Self::Id(id));
        }
        serde_json::from_str(value)
            .map(Self::Address)
            .map_err(|_| "expected a workspace UUID or the previous page's JSON next cursor")
    }
}

pub(crate) fn print_page(list: &ListWorkspacesResult, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(list)?);
    } else {
        for workspace in &list.workspaces {
            println!("{}", line(workspace));
        }
        if let Some(next) = list.next {
            println!("next={}", serde_json::to_string(&next)?);
        }
    }
    Ok(())
}

pub(crate) fn address(hub: HubId, id: Uuid) -> Result<WorkspaceAddress> {
    let workspace_id = WorkspaceId::from_uuid(id).map_err(|_| anyhow!(INVALID_SELECTOR))?;
    Ok(WorkspaceAddress {
        hub_id: hub,
        workspace_id,
    })
}

fn labels(values: &[String]) -> Result<BTreeMap<String, String>> {
    let mut labels = BTreeMap::new();
    for value in values {
        let (key, value) = value
            .split_once('=')
            .ok_or_else(|| anyhow!(INVALID_LABEL))?;
        labels.insert(key.to_owned(), value.to_owned());
    }
    Ok(labels)
}

fn membership(args: &UpdateArgs) -> Result<Option<MembershipChange>> {
    let bind = !args.bind_terminals.is_empty();
    let unbind = !args.unbind_terminals.is_empty();
    if bind && unbind {
        return Err(anyhow!(MIXED_MEMBERSHIP));
    }
    Ok(if bind {
        Some(MembershipChange::Bind {
            terminals: args.bind_terminals.clone(),
        })
    } else if unbind {
        Some(MembershipChange::Unbind {
            terminals: args.unbind_terminals.clone(),
        })
    } else {
        None
    })
}

pub(crate) async fn call(client: &mut LocalClient, hub: HubId, command: Rpc) -> Result<Reply> {
    let deadline = Instant::now() + command.operation_timeout(RPC_TIMEOUT);
    Ok(client.call(hub, command, deadline).await?)
}

pub(crate) fn print(workspace: &WorkspaceSummary, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(workspace)?);
    } else {
        println!("{}", line(workspace));
    }
    Ok(())
}

fn line(workspace: &WorkspaceSummary) -> String {
    format!(
        "{} {} {} revision={}",
        workspace.name, workspace.address.workspace_id, workspace.root, workspace.revision
    )
}

pub(crate) fn unexpected() -> anyhow::Error {
    anyhow!("the Hub returned an unexpected reply")
}
