use crate::{
    caller_context,
    creation_request::runtime_run_request,
    launch::{address, create_finalized},
    workspace::WorkspaceService,
};
use yolo_client::Client;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{PublicError, TerminalRunRequest, TerminalRunResult};

/// Create a shell with one bounded launch input and finalize it before
/// returning. The Worker delivers the input only after its hook ingress is
/// armed, so provider startup callbacks cannot race the launch.
pub async fn terminal_run(
    hub: HubId,
    client: &Client,
    workspaces: Option<&WorkspaceService>,
    request: &TerminalRunRequest,
) -> Result<Reply, PublicError> {
    let context =
        crate::caller_context::caller_context(hub, workspaces, request.fields().workspace_id, None)
            .await?;
    let workspace = caller_context::workspace_address(hub, context);
    let _reservation = match (workspaces, workspace) {
        (Some(service), Some(address)) => Some(service.reserve_creation(address).await?),
        _ => None,
    };
    let created = create_finalized(client, &runtime_run_request(request, context)).await?;
    if let (Some(service), Some(workspace)) = (workspaces, workspace) {
        service
            .bind_terminal(workspace, created.terminal_id)
            .await?;
    }
    Ok(Reply::TerminalRun {
        run: TerminalRunResult {
            address: address(hub, created.terminal_id)?,
        },
    })
}
