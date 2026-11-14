use crate::arguments::{WorktreeCommand, WorktreeOptions};
use crate::commands::{address, call, print, resolve, unexpected};
use crate::connection::{self, Connection};
use crate::constants::*;
use anyhow::{Result, anyhow};
use uuid::Uuid;
use yolo_hub_client::LocalClient;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command as Rpc, Reply};
use yolo_hub_workspace_protocol::{
    CreateWorktreeFields, CreateWorktreeRequest, ListWorkspacesResult, ListWorktreesFields,
    ListWorktreesRequest, RemoveWorktreeFields, RemoveWorktreeRequest,
    WORKSPACE_PROTOCOL_SCHEMA_VERSION, WorkspaceAddress, WorkspaceKind, WorkspaceSummary,
};

pub async fn run(options: WorktreeOptions) -> Result<()> {
    let mut client = connection::connect(&Connection {
        socket: options.socket,
        hub: options.hub,
        local_profile: options.local_profile,
    })
    .await?;
    let hub = client.hello().hub_id;
    match options.command {
        WorktreeCommand::Create(args) => {
            if !args.trust_repository {
                return Err(anyhow!(TRUST_REQUIRED));
            }
            let parent = resolve(&mut client, hub, &args.parent).await?;
            let request = CreateWorktreeRequest::new(CreateWorktreeFields {
                schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
                request_id: Uuid::new_v4(),
                workspace_id: Uuid::new_v4(),
                parent: parent.address,
                name: args.name,
                branch: args.branch,
                base: args.base,
                trust_repository: true,
            })?;
            let destination = parent.address.hub_id;
            let workspace = match call(&mut client, destination, Rpc::CreateWorktree { request }).await? {
                Reply::Workspace { workspace } => workspace,
                _ => return Err(unexpected()),
            };
            print(&workspace, args.json)
        }
        WorktreeCommand::Remove(args) => {
            let workspace = resolve(&mut client, hub, &args.selector).await?;
            if workspace.kind != WorkspaceKind::Worktree {
                return Err(anyhow!(INVALID_WORKTREE));
            }
            let request = RemoveWorktreeRequest::new(RemoveWorktreeFields {
                schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
                request_id: Uuid::new_v4(),
                address: workspace.address,
            })?;
            let destination = workspace.address.hub_id;
            match call(&mut client, destination, Rpc::RemoveWorktree { change: request }).await? {
                Reply::Completed {} => {
                    println!("{REMOVED} {}", workspace.name);
                    Ok(())
                }
                _ => Err(unexpected()),
            }
        }
        WorktreeCommand::List(args) => {
            let parent = resolve(&mut client, hub, &args.parent).await?;
            let destination = parent.address.hub_id;
            let after = args.after.map(|id| address(destination, id)).transpose()?;
            let request = ListWorktreesRequest::new(ListWorktreesFields {
                schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
                parent: parent.address,
                after,
                limit: args.limit,
            })?;
            let list = match call(&mut client, destination, Rpc::ListWorktrees { request }).await? {
                Reply::Workspaces { list } => list,
                _ => return Err(unexpected()),
            };
            output(&list, args.json)
        }
    }
}

fn output(list: &ListWorkspacesResult, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(list)?);
    } else {
        for workspace in &list.workspaces {
            println!("{}", line(workspace));
        }
    }
    Ok(())
}

fn line(workspace: &WorkspaceSummary) -> String {
    format!(
        "{} {} {} revision={}",
        workspace.name, workspace.address.workspace_id, workspace.root, workspace.revision
    )
}

#[allow(dead_code)]
fn _keep(_: &mut LocalClient, _: HubId, _: WorkspaceAddress) {}
