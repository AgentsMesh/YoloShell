use crate::*;
use uuid::Uuid;
use yolo_hub_terminal_protocol::PublicError;

mod membership;
mod store;

fn fixture() -> (tempfile::TempDir, HubId, WorkspaceStore) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("workspaces");
    let hub = HubId::new();
    let store = WorkspaceStore::open(
        yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(),
        hub,
    )
    .unwrap();
    (root, hub, store)
}

fn create(hub: HubId, id: Uuid, name: &str) -> CreateWorkspaceRequest {
    CreateWorkspaceRequest::new(yolo_hub_workspace_protocol::CreateWorkspaceFields {
        schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        workspace_id: id,
        name: name.into(),
        root: format!("/tmp/{name}"),
        labels: Default::default(),
    })
    .unwrap()
}

fn list(limit: u16, after: Option<WorkspaceAddress>) -> ListWorkspacesRequest {
    ListWorkspacesRequest::new(yolo_hub_workspace_protocol::ListWorkspacesFields {
        schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        after,
        limit,
    })
    .unwrap()
}
