use super::super::rpc::request_checked;
use super::super::runtime_guard::DetachedRuntimeGuard;
use anyhow::{Context, Result, ensure};
use yolo_runtime_rpc::RuntimeConnection;
use yolo_runtime::host::{ListSessionsRequest, ListSessionsResult};
use yolo_runtime::protocol::method;

const INCLUDE_ARCHIVED_SESSIONS: bool = true;
const REQUIRE_COMPLETE_INVENTORY: bool = true;
const INVENTORY_REQUEST_CONTEXT: &str = "enumerate authenticated detached Host runtimes";
const INCOMPLETE_INVENTORY_DETAIL: &str =
    "detached Host did not attest a complete runtime inventory";
const INVENTORY_BIND_CONTEXT: &str = "bind authenticated detached Host runtime inventory";

pub(in super::super) async fn bind_runtime_inventory(
    stream: &mut RuntimeConnection,
    guard: &mut DetachedRuntimeGuard,
) -> Result<()> {
    let inventory: ListSessionsResult = request_checked(
        stream,
        method::LIST_SESSIONS,
        &ListSessionsRequest {
            include_archived: INCLUDE_ARCHIVED_SESSIONS,
            require_complete: REQUIRE_COMPLETE_INVENTORY,
        },
    )
    .await
    .context(INVENTORY_REQUEST_CONTEXT)?;
    ensure!(inventory.complete, INCOMPLETE_INVENTORY_DETAIL);
    guard
        .bind_authenticated_inventory(&inventory.sessions)
        .context(INVENTORY_BIND_CONTEXT)
}
