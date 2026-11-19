use crate::constants::RPC_TIMEOUT;
use tokio::time::Instant;
use yolo_hub_client::{ClientError, EndpointClient};
use yolo_hub_rpc_protocol::Command;

/// Probe the exact connection whose next operation relies on its liveness.
/// A separate socket's successful Hello says nothing about this stream.
pub(crate) async fn probe(client: &mut EndpointClient) -> Result<(), ClientError> {
    client
        .call(
            client.hello().hub_id,
            Command::Hello {},
            Instant::now() + RPC_TIMEOUT,
        )
        .await
        .map(|_| ())
}
