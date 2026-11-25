use crate::fixtures::*;
use yolo_hub_rpc_protocol::{Command, Reply};

pub fn samples() -> Vec<(Command, Reply)> {
    vec![
        (
            Command::CreateWorkspace {
                request: create_workspace(),
            },
            Reply::Workspace {
                workspace: workspace_summary(),
            },
        ),
        (
            Command::UpdateWorkspace {
                change: update_workspace(),
            },
            Reply::Workspace {
                workspace: {
                    let mut workspace = workspace_summary();
                    workspace.revision = 2;
                    workspace
                },
            },
        ),
        (
            Command::GetWorkspace {
                request: get_workspace(),
            },
            Reply::Workspace {
                workspace: workspace_summary(),
            },
        ),
        (
            Command::ListWorkspaces {
                request: list_workspaces(),
            },
            Reply::Workspaces {
                list: yolo_hub_workspace_protocol::ListWorkspacesResult {
                    workspaces: vec![workspace_summary()],
                    next: None,
                },
            },
        ),
        (
            Command::CreateWorktree {
                request: create_worktree(),
            },
            Reply::Workspace {
                workspace: worktree_summary(),
            },
        ),
        (
            Command::RemoveWorktree {
                change: remove_worktree(),
            },
            Reply::Completed {},
        ),
        (
            Command::ListWorktrees {
                request: list_worktrees(),
            },
            Reply::Workspaces {
                list: yolo_hub_workspace_protocol::ListWorkspacesResult {
                    workspaces: vec![worktree_summary()],
                    next: None,
                },
            },
        ),
    ]
}
