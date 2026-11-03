use crate::errors::map_error;
use yolo_client::Client;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_workspace_protocol::{
    CreateWorkspaceFields, CreateWorkspaceRequest, GetWorkspaceRequest, ListWorkspacesRequest,
    WorkspaceCapabilities, WorkspaceSelector, WorkspaceSummary,
};
use yolo_hub_workspace_store::{StoreError, WorkspaceStore};
use yolo_paths::directory::PrivateDirectory;
use yolo_runtime_protocol::WorkspaceInspectRequest;

/// Hub-owned workspace metadata. The Host validates the root before the store
/// accepts it; reads never touch the file system.
pub struct WorkspaceService {
    client: Client,
    store: tokio::sync::Mutex<WorkspaceStore>,
}

impl WorkspaceService {
    pub fn shared(
        directory: PrivateDirectory,
        hub: HubId,
        client: Client,
    ) -> Result<std::sync::Arc<Self>, PublicError> {
        Self::open(directory, hub, client).map(std::sync::Arc::new)
    }

    pub fn open(
        directory: PrivateDirectory,
        hub: HubId,
        client: Client,
    ) -> Result<Self, PublicError> {
        Ok(Self {
            client,
            store: tokio::sync::Mutex::new(
                WorkspaceStore::open(directory, hub).map_err(StoreError::into_public)?,
            ),
        })
    }

    pub(crate) fn client(&self) -> &Client {
        &self.client
    }

    pub(crate) fn store(&self) -> &tokio::sync::Mutex<WorkspaceStore> {
        &self.store
    }

    pub async fn create(
        &self,
        request: &CreateWorkspaceRequest,
    ) -> Result<WorkspaceSummary, PublicError> {
        let inspected = self
            .client
            .workspace_inspect(&WorkspaceInspectRequest {
                root: request.fields().root.clone(),
            })
            .await
            .map_err(map_error)?;
        let fields = CreateWorkspaceFields {
            schema_version: request.fields().schema_version,
            request_id: request.fields().request_id,
            workspace_id: request.fields().workspace_id,
            name: request.fields().name.clone(),
            root: inspected.canonical_root,
            labels: request.fields().labels.clone(),
        };
        let canonical = CreateWorkspaceRequest::new(fields)?;
        let mut summary = self
            .store
            .lock()
            .await
            .create(&canonical)
            .map_err(StoreError::into_public)?;
        summary.capabilities = self.capabilities(&summary.root).await;
        Ok(summary)
    }

    pub async fn create_worktree(
        &self,
        request: &yolo_hub_workspace_protocol::CreateWorktreeRequest,
    ) -> Result<WorkspaceSummary, PublicError> {
        crate::workspace_worktree::create(self, request).await
    }

    pub async fn remove_worktree(
        &self,
        request: &yolo_hub_workspace_protocol::RemoveWorktreeRequest,
    ) -> Result<(), PublicError> {
        crate::workspace_worktree::remove(self, request).await
    }

    pub async fn list_worktrees(
        &self,
        request: &yolo_hub_workspace_protocol::ListWorktreesRequest,
    ) -> Result<yolo_hub_workspace_protocol::ListWorkspacesResult, PublicError> {
        crate::workspace_worktree::list(self, request).await
    }

    pub(crate) async fn capabilities(
        &self,
        root: &str,
    ) -> yolo_hub_workspace_protocol::WorkspaceCapabilities {
        crate::workspace_worktree::capabilities(&self.client, root).await
    }

    pub async fn get(
        &self,
        request: &GetWorkspaceRequest,
    ) -> Result<WorkspaceSummary, PublicError> {
        let mut summary = {
            let store = self.store.lock().await;
            match &request.fields().selector {
                WorkspaceSelector::Address(address) => store.get(*address),
                WorkspaceSelector::Name(name) => store.find_name(name),
            }
            .map_err(StoreError::into_public)?
            .ok_or(PublicError::NotFound)?
        };
        summary.capabilities = self.capabilities(&summary.root).await;
        Ok(summary)
    }

    pub async fn list(
        &self,
        request: &ListWorkspacesRequest,
    ) -> Result<yolo_hub_workspace_protocol::ListWorkspacesResult, PublicError> {
        let (mut workspaces, next) = self
            .store
            .lock()
            .await
            .list(request)
            .map_err(StoreError::into_public)?;
        let mut cache: std::collections::BTreeMap<String, WorkspaceCapabilities> =
            std::collections::BTreeMap::new();
        for workspace in &mut workspaces {
            let capabilities = match cache.get(&workspace.root) {
                Some(capabilities) => capabilities.clone(),
                None => {
                    let capabilities = self.capabilities(&workspace.root).await;
                    cache.insert(workspace.root.clone(), capabilities.clone());
                    capabilities
                }
            };
            workspace.capabilities = capabilities;
        }
        self.apply_counts(&mut workspaces).await?;
        Ok(yolo_hub_workspace_protocol::ListWorkspacesResult { workspaces, next })
    }
}

/// Dispatch the workspace command group. The backend keeps only the service
/// handle and one route arm; command-to-reply mapping stays in the platform
/// crate so the grandfathered backend package does not grow.
pub async fn workspace_dispatch(
    service: Option<&WorkspaceService>,
    command: &Command,
) -> Result<Reply, PublicError> {
    let service = service.ok_or(PublicError::Unsupported)?;
    let mut reply = match command {
        Command::CreateWorkspace { request } => Reply::Workspace {
            workspace: service.create(request).await?,
        },
        Command::UpdateWorkspace { change } => Reply::Workspace {
            workspace: crate::workspace_membership::update(service, change).await?,
        },
        Command::GetWorkspace { request } => Reply::Workspace {
            workspace: service.get(request).await?,
        },
        Command::ListWorkspaces { request } => Reply::Workspaces {
            list: service.list(request).await?,
        },
        Command::CreateWorktree { request } => Reply::Workspace {
            workspace: service.create_worktree(request).await?,
        },
        Command::RemoveWorktree { change } => {
            service.remove_worktree(change).await?;
            Reply::Completed {}
        }
        Command::ListWorktrees { request } => Reply::Workspaces {
            list: service.list_worktrees(request).await?,
        },
        _ => return Err(PublicError::Unsupported),
    };
    match &mut reply {
        Reply::Workspace { workspace } => {
            service
                .apply_counts(std::slice::from_mut(workspace))
                .await?
        }
        _ => {}
    }
    Ok(reply)
}
