use crate::commands::{call, unexpected};
use crate::constants::INVALID_SELECTOR;
use anyhow::{anyhow, Result};
use uuid::Uuid;
use yolo_hub_client::LocalClient;
use yolo_hub_identity::{HubId, WorkspaceAddress};
use yolo_hub_rpc_protocol::{Command as Rpc, Reply};
use yolo_hub_workspace_protocol::{
    GetWorkspaceFields, GetWorkspaceRequest, ListWorkspacesFields, ListWorkspacesRequest,
    WORKSPACE_PROTOCOL_SCHEMA_VERSION, MAX_WORKSPACE_PAGE, WorkspaceSelector, WorkspaceSummary,
};

/// Finds a local or federated workspace address by id or name.
pub(crate) async fn address_of(
    client: &mut LocalClient,
    hub: HubId,
    selector: &str,
) -> Result<WorkspaceAddress> {
    let id = Uuid::parse_str(selector).ok();
    let name = if id.is_none() && !selector.trim().is_empty() {
        Some(selector.to_owned())
    } else {
        None
    };
    if id.is_none() && name.is_none() {
        return Err(anyhow!(INVALID_SELECTOR));
    }
    let mut after = None;
    loop {
        let request = ListWorkspacesRequest::new(ListWorkspacesFields {
            schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            after,
            limit: MAX_WORKSPACE_PAGE as u16,
        })?;
        let Reply::Workspaces { list } = call(client, hub, Rpc::ListWorkspaces { request }).await?
        else {
            return Err(unexpected());
        };
        let found = list
            .workspaces
            .into_iter()
            .find(|workspace| match (&id, &name) {
                (Some(id), _) => workspace.address.workspace_id.as_uuid() == *id,
                (None, Some(name)) => workspace.name.eq_ignore_ascii_case(name),
                (None, None) => false,
            });
        if let Some(workspace) = found {
            return Ok(workspace.address);
        }
        let Some(next) = list.next else {
            return Err(anyhow!(INVALID_SELECTOR));
        };
        after = Some(next);
    }
}

/// Resolves a local or federated workspace, then fetches it from its owner Hub.
pub(crate) async fn resolve(
    client: &mut LocalClient,
    hub: HubId,
    selector: &str,
) -> Result<WorkspaceSummary> {
    let address = address_of(client, hub, selector).await?;
    let request = GetWorkspaceRequest::new(GetWorkspaceFields {
        schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        selector: WorkspaceSelector::Address(address),
    })?;
    let destination = address.hub_id;
    match call(client, destination, Rpc::GetWorkspace { request }).await? {
        Reply::Workspace { workspace } => Ok(workspace),
        _ => Err(unexpected()),
    }
}
