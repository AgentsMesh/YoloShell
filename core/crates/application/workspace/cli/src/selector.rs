use crate::commands::{call, unexpected};
use crate::constants::INVALID_SELECTOR;
use anyhow::{Result, anyhow};
use uuid::Uuid;
use yolo_hub_client::LocalClient;
use yolo_hub_identity::{HubId, WorkspaceAddress};
use yolo_hub_rpc_protocol::{Command as Rpc, Reply};
use yolo_hub_workspace_protocol::{
    GetWorkspaceFields, GetWorkspaceRequest, ListWorkspacesFields, ListWorkspacesRequest,
    MAX_WORKSPACE_PAGE, WORKSPACE_PROTOCOL_SCHEMA_VERSION, WorkspaceSelector, WorkspaceSummary,
};

#[derive(Default)]
struct Matches {
    exact: Vec<WorkspaceAddress>,
    folded: Vec<WorkspaceAddress>,
}

impl Matches {
    fn consider(
        &mut self,
        address: WorkspaceAddress,
        name: &str,
        selector: &str,
        folded: &str,
        id: Option<Uuid>,
    ) {
        if let Some(id) = id {
            if address.workspace_id.as_uuid() == id {
                self.exact.push(address);
            }
        } else if name == selector {
            self.exact.push(address);
        } else if name.to_lowercase() == folded {
            self.folded.push(address);
        }
    }

    fn resolve(self) -> Result<WorkspaceAddress> {
        let addresses = if self.exact.is_empty() {
            self.folded
        } else {
            self.exact
        };
        match addresses.as_slice() {
            [address] => Ok(*address),
            [] => Err(anyhow!(INVALID_SELECTOR)),
            _ => Err(anyhow!(
                "workspace selector matches multiple workspaces; use a unique workspace id"
            )),
        }
    }
}

/// Finds a local or federated workspace address by id or name.
pub(crate) async fn address_of(
    client: &mut LocalClient,
    hub: HubId,
    selector: &str,
) -> Result<WorkspaceAddress> {
    let id = Uuid::parse_str(selector).ok();
    if selector.trim().is_empty() {
        return Err(anyhow!(INVALID_SELECTOR));
    }
    let folded = selector.to_lowercase();
    let mut matches = Matches::default();
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
        for workspace in list.workspaces {
            matches.consider(workspace.address, &workspace.name, selector, &folded, id);
        }
        let Some(next) = list.next else {
            return matches.resolve();
        };
        after = Some(next);
    }
}

#[cfg(test)]
#[path = "../unit_tests/selector.rs"]
mod tests;

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
