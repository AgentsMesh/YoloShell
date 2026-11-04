use super::super::router::CodexCommittedRoute;
use super::cursor::ProjectionCursor;
use super::forwarder::ProjectionForwarders;
use crate::worker::WorkerState;
use anyhow::Result;

pub(super) async fn seed(
    state: &WorkerState,
    cursor: &mut ProjectionCursor,
    forwarders: &mut ProjectionForwarders,
    snapshot: Vec<CodexCommittedRoute>,
    sender: &tokio::sync::mpsc::Sender<super::forwarder::ProjectionInput>,
) -> Result<()> {
    for route in snapshot {
        if let Some(input) = forwarders.install_seeded(route, sender.clone()) {
            super::consume(state, cursor, input).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/projection/projection_snapshot_tests.rs"]
mod tests;
