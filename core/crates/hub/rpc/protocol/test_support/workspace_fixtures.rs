use crate::fixtures::constants::*;
use crate::fixtures::{hub, id};

const WORKSPACE_PAGE_FIXTURE: u16 = 16;

pub fn workspace_summary() -> yolo_hub_workspace_protocol::WorkspaceSummary {
    yolo_hub_workspace_protocol::WorkspaceSummary {
        address: yolo_hub_workspace_protocol::WorkspaceAddress {
            hub_id: hub(HUB_ID),
            workspace_id: yolo_hub_identity::WorkspaceId::from_uuid(id(SESSION_ID)).unwrap(),
        },
        revision: 1,
        name: "workspace".into(),
        root: DIRECTORY.into(),
        kind: yolo_hub_workspace_protocol::WorkspaceKind::Directory,
        parent: None,
        labels: Default::default(),
        member_terminals: 0,
        member_runs: 0,
        capabilities: yolo_hub_workspace_protocol::WorkspaceCapabilities::unsupported(
            "git executor is not configured",
        ),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

pub fn create_workspace() -> yolo_hub_workspace_protocol::CreateWorkspaceRequest {
    yolo_hub_workspace_protocol::CreateWorkspaceRequest::new(
        yolo_hub_workspace_protocol::CreateWorkspaceFields {
            schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            request_id: id(REQUEST_ID),
            workspace_id: id(SESSION_ID),
            name: "workspace".into(),
            root: DIRECTORY.into(),
            labels: Default::default(),
        },
    )
    .unwrap()
}

pub fn update_workspace() -> yolo_hub_workspace_protocol::UpdateWorkspaceRequest {
    yolo_hub_workspace_protocol::UpdateWorkspaceRequest::new(
        yolo_hub_workspace_protocol::UpdateWorkspaceFields {
            schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            request_id: id(REQUEST_ID),
            address: workspace_summary().address,
            expected_revision: 1,
            name: Some("workspace".into()),
            labels: None,
            members: None,
        },
    )
    .unwrap()
}

pub fn get_workspace() -> yolo_hub_workspace_protocol::GetWorkspaceRequest {
    yolo_hub_workspace_protocol::GetWorkspaceRequest::new(
        yolo_hub_workspace_protocol::GetWorkspaceFields {
            schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            selector: yolo_hub_workspace_protocol::WorkspaceSelector::Address(
                workspace_summary().address,
            ),
        },
    )
    .unwrap()
}

pub fn list_workspaces() -> yolo_hub_workspace_protocol::ListWorkspacesRequest {
    yolo_hub_workspace_protocol::ListWorkspacesRequest::new(
        yolo_hub_workspace_protocol::ListWorkspacesFields {
            schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            after: None,
            limit: WORKSPACE_PAGE_FIXTURE,
        },
    )
    .unwrap()
}

pub fn worktree_summary() -> yolo_hub_workspace_protocol::WorkspaceSummary {
    let mut summary = workspace_summary();
    summary.address.workspace_id =
        yolo_hub_identity::WorkspaceId::from_uuid(id(SECOND_SESSION_ID)).unwrap();
    summary.name = "worktree".into();
    summary.root = "/workspace/worktree".into();
    summary.kind = yolo_hub_workspace_protocol::WorkspaceKind::Worktree;
    summary.parent = Some(workspace_summary().address);
    summary
}

pub fn create_worktree() -> yolo_hub_workspace_protocol::CreateWorktreeRequest {
    yolo_hub_workspace_protocol::CreateWorktreeRequest::new(
        yolo_hub_workspace_protocol::CreateWorktreeFields {
            schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            request_id: id(REQUEST_ID),
            workspace_id: id(SECOND_SESSION_ID),
            parent: workspace_summary().address,
            name: "worktree".into(),
            branch: Some("feature".into()),
            base: None,
            trust_repository: true,
        },
    )
    .unwrap()
}

pub fn remove_worktree() -> yolo_hub_workspace_protocol::RemoveWorktreeRequest {
    yolo_hub_workspace_protocol::RemoveWorktreeRequest::new(
        yolo_hub_workspace_protocol::RemoveWorktreeFields {
            schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            request_id: id(REQUEST_ID),
            address: worktree_summary().address,
        },
    )
    .unwrap()
}

pub fn list_worktrees() -> yolo_hub_workspace_protocol::ListWorktreesRequest {
    yolo_hub_workspace_protocol::ListWorktreesRequest::new(
        yolo_hub_workspace_protocol::ListWorktreesFields {
            schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            parent: workspace_summary().address,
            after: None,
            limit: WORKSPACE_PAGE_FIXTURE,
        },
    )
    .unwrap()
}
