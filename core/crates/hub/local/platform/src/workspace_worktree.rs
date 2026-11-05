use crate::errors::map_error;
use crate::workspace::WorkspaceService;
use yolo_client::Client;
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_workspace_protocol::{
    CreateWorktreeRequest, GitCapability, ListWorkspacesResult, ListWorktreesRequest,
    RemoveWorktreeRequest, WorkspaceCapabilities, WorkspaceKind, WorkspaceSummary,
    WorktreeCapability,
};
use yolo_runtime_protocol::{GitProbe, WorktreeCreateRequest, WorktreeRemoveRequest};

pub(crate) async fn create(
    service: &WorkspaceService,
    request: &CreateWorktreeRequest,
) -> Result<WorkspaceSummary, PublicError> {
    let fields = request.fields();
    let parent = service
        .store()
        .lock()
        .await
        .get(fields.parent)
        .map_err(yolo_hub_workspace_store::StoreError::into_public)?
        .ok_or(PublicError::NotFound)?;
    if parent.kind != WorkspaceKind::Directory {
        return Err(PublicError::Conflict);
    }
    let address = yolo_hub_identity::WorkspaceAddress {
        hub_id: fields.parent.hub_id,
        workspace_id: yolo_hub_identity::WorkspaceId::from_uuid(fields.workspace_id)
            .map_err(|_| PublicError::InvalidRequest)?,
    };
    let (existing, name_taken) = {
        let store = service.store().lock().await;
        (
            store
                .get(address)
                .map_err(yolo_hub_workspace_store::StoreError::into_public)?,
            store
                .find_name(&fields.name)
                .map_err(yolo_hub_workspace_store::StoreError::into_public)?
                .is_some(),
        )
    };
    if let Some(mut summary) = existing {
        if summary.name == fields.name
            && summary.parent == Some(fields.parent)
            && summary.kind == WorkspaceKind::Worktree
        {
            summary.capabilities = capabilities(service.client(), &summary.root).await;
            return Ok(summary);
        }
        return Err(PublicError::Conflict);
    }
    if name_taken {
        return Err(PublicError::Conflict);
    }
    let created = service
        .client()
        .worktree_create(&WorktreeCreateRequest {
            repository_root: parent.root.clone(),
            worktree_id: fields.workspace_id.to_string(),
            branch: fields.branch.clone(),
            base: fields.base.clone(),
            trust_repository: fields.trust_repository,
        })
        .await
        .map_err(map_error)?;
    let stored = service
        .store()
        .lock()
        .await
        .create_worktree(request, created.path.clone());
    let mut summary = match stored {
        Ok(summary) => summary,
        Err(error) => {
            // The Host created an exact path in this request. Roll it back;
            // the rollback outcome never upgrades the original failure.
            let _ = service
                .client()
                .worktree_remove(&WorktreeRemoveRequest {
                    repository_root: parent.root,
                    path: created.path,
                })
                .await;
            return Err(error.into_public());
        }
    };
    summary.capabilities = capabilities(service.client(), &summary.root).await;
    Ok(summary)
}

pub(crate) async fn remove(
    service: &WorkspaceService,
    request: &RemoveWorktreeRequest,
) -> Result<(), PublicError> {
    let address = request.fields().address;
    // Keep the store lock across the Host effect so a membership bind cannot
    // race the checkout deletion.
    let mut store = service.store().lock().await;
    let workspace = store
        .get(address)
        .map_err(yolo_hub_workspace_store::StoreError::into_public)?
        .ok_or(PublicError::NotFound)?;
    if workspace.kind != WorkspaceKind::Worktree {
        return Err(PublicError::Conflict);
    }
    let parent = workspace.parent.ok_or(PublicError::Internal)?;
    let repository_root = store
        .get(parent)
        .map_err(yolo_hub_workspace_store::StoreError::into_public)?
        .ok_or(PublicError::NotFound)?
        .root;
    store
        .ensure_removable(address)
        .map_err(yolo_hub_workspace_store::StoreError::into_public)?;
    service
        .client()
        .worktree_remove(&WorktreeRemoveRequest {
            repository_root,
            path: workspace.root,
        })
        .await
        .map_err(map_error)?;
    store
        .remove(address)
        .map_err(yolo_hub_workspace_store::StoreError::into_public)
}

pub(crate) async fn list(
    service: &WorkspaceService,
    request: &ListWorktreesRequest,
) -> Result<ListWorkspacesResult, PublicError> {
    let (mut workspaces, next) = service
        .store()
        .lock()
        .await
        .list_worktrees(request)
        .map_err(yolo_hub_workspace_store::StoreError::into_public)?;
    service.apply_counts(&mut workspaces).await?;
    // Worktree capabilities belong to the parent repository: one probe covers
    // every row instead of re-inspecting each checkout.
    let parent_root = if workspaces.is_empty() {
        None
    } else {
        let store = service.store().lock().await;
        store
            .get(request.fields().parent)
            .ok()
            .flatten()
            .map(|parent| parent.root)
    };
    let capabilities = match parent_root {
        Some(root) => Some(capabilities(service.client(), &root).await),
        None => None,
    };
    for workspace in &mut workspaces {
        if let Some(capabilities) = &capabilities {
            workspace.capabilities = capabilities.clone();
        }
    }
    Ok(ListWorkspacesResult { workspaces, next })
}

pub(crate) async fn capabilities(client: &Client, root: &str) -> WorkspaceCapabilities {
    let inspected = client
        .workspace_inspect(&yolo_runtime_protocol::WorkspaceInspectRequest { root: root.into() })
        .await
        .ok();
    match inspected.map(|result| result.git) {
        Some(GitProbe::Available { version }) => WorkspaceCapabilities {
            git: GitCapability::Available { version },
            worktree: WorktreeCapability::Available,
        },
        Some(GitProbe::Unavailable { reason }) => WorkspaceCapabilities::unsupported(&reason),
        None => WorkspaceCapabilities::unsupported("workspace capability inspection failed"),
    }
}
