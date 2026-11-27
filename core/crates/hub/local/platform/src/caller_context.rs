use crate::workspace::WorkspaceService;
use yolo_hub_identity::{HubId, WorkspaceAddress, WorkspaceId};
use yolo_hub_terminal_protocol::PublicError;
use yolo_terminal_model::TerminalCallerContext;

/// A workspace-scoped creation carries Hub-owned caller identity. Public
/// requests only name a workspace id; the Hub proves it exists before launch.
pub(crate) async fn caller_context(
    hub: HubId,
    workspaces: Option<&WorkspaceService>,
    workspace_id: Option<uuid::Uuid>,
    run_id: Option<uuid::Uuid>,
) -> Result<Option<TerminalCallerContext>, PublicError> {
    let Some(workspace_id) = workspace_id else {
        return Ok(Some(TerminalCallerContext {
            hub_id: hub.as_uuid(),
            workspace_id: None,
            run_id,
        }));
    };
    let workspace_id = yolo_hub_identity::WorkspaceId::from_uuid(workspace_id)
        .map_err(|_| PublicError::InvalidRequest)?;
    let address = yolo_hub_workspace_protocol::WorkspaceAddress {
        hub_id: hub,
        workspace_id,
    };
    let workspaces = workspaces.ok_or(PublicError::Unsupported)?;
    if !workspaces.exists(address).await? {
        return Err(PublicError::NotFound);
    }
    Ok(Some(TerminalCallerContext {
        hub_id: hub.as_uuid(),
        workspace_id: Some(workspace_id.as_uuid()),
        run_id,
    }))
}

/// The durable workspace address derived from a validated caller context.
pub(crate) fn workspace_address(
    hub: HubId,
    context: Option<TerminalCallerContext>,
) -> Option<WorkspaceAddress> {
    let workspace_id = WorkspaceId::from_uuid(context?.workspace_id?).ok()?;
    Some(WorkspaceAddress {
        hub_id: hub,
        workspace_id,
    })
}
