use crate::{Command, Reply};

/// Correlation for the workspace and worktree groups.
pub(crate) fn matches_target(reply: &Reply, command: &Command) -> Option<bool> {
    Some(match (reply, command) {
        (Reply::Workspace { workspace }, Command::CreateWorkspace { request }) => {
            workspace.address.workspace_id.as_uuid() == request.fields().workspace_id
        }
        (Reply::Workspace { workspace }, Command::UpdateWorkspace { change }) => {
            workspace.address == change.fields().address
                && workspace.revision == change.fields().expected_revision.saturating_add(1)
        }
        (Reply::Workspace { workspace }, Command::GetWorkspace { request }) => {
            match &request.fields().selector {
                yolo_hub_workspace_protocol::WorkspaceSelector::Address(address) => {
                    workspace.address == *address
                }
                yolo_hub_workspace_protocol::WorkspaceSelector::Name(name) => {
                    workspace.name.eq_ignore_ascii_case(name)
                }
            }
        }
        (Reply::Workspaces { list }, Command::ListWorkspaces { request }) => {
            let limit = request.fields().limit as usize;
            list.workspaces.len() <= limit
                && list
                    .workspaces
                    .windows(2)
                    .all(|pair| pair[0].address < pair[1].address)
                && list.next.is_none_or(|next| {
                    list.workspaces
                        .last()
                        .is_some_and(|workspace| workspace.address == next)
                })
        }
        (Reply::Workspace { workspace }, Command::CreateWorktree { request }) => {
            workspace.address.workspace_id.as_uuid() == request.fields().workspace_id
                && workspace.parent == Some(request.fields().parent)
        }
        (Reply::Completed {}, Command::RemoveWorktree { .. }) => true,
        (Reply::Workspaces { list }, Command::ListWorktrees { request }) => {
            let limit = request.fields().limit as usize;
            list.workspaces.len() <= limit
                && list
                    .workspaces
                    .windows(2)
                    .all(|pair| pair[0].address < pair[1].address)
                && list
                    .workspaces
                    .iter()
                    .all(|workspace| workspace.parent == Some(request.fields().parent))
                && list.next.is_none_or(|next| {
                    list.workspaces
                        .last()
                        .is_some_and(|workspace| workspace.address == next)
                })
        }
        _ => return None,
    })
}
