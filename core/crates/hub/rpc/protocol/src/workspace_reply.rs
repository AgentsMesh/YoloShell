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
                    workspace.name.to_lowercase() == name.to_lowercase()
                }
            }
        }
        (Reply::Workspaces { list }, Command::ListWorkspaces { request }) => {
            valid_page(list, request.fields().after, request.fields().limit)
        }
        (Reply::Workspace { workspace }, Command::CreateWorktree { request }) => {
            workspace.address.workspace_id.as_uuid() == request.fields().workspace_id
                && workspace.parent == Some(request.fields().parent)
        }
        (Reply::Completed {}, Command::RemoveWorktree { .. }) => true,
        (Reply::Workspaces { list }, Command::ListWorktrees { request }) => {
            valid_page(list, request.fields().after, request.fields().limit)
                && list
                    .workspaces
                    .iter()
                    .all(|workspace| workspace.parent == Some(request.fields().parent))
        }
        _ => return None,
    })
}

fn valid_page(
    list: &yolo_hub_workspace_protocol::ListWorkspacesResult,
    after: Option<yolo_hub_identity::WorkspaceAddress>,
    limit: u16,
) -> bool {
    list.workspaces.len() <= usize::from(limit)
        && list
            .workspaces
            .first()
            .is_none_or(|first| after.is_none_or(|after| first.address > after))
        && list
            .workspaces
            .windows(2)
            .all(|pair| pair[0].address < pair[1].address)
        && list.next.is_none_or(|next| {
            list.workspaces
                .last()
                .is_some_and(|last| last.address == next)
        })
}
