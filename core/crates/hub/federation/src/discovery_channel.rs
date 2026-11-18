use crate::registry::Link;
use anyhow::Result;
use tokio::time::Instant;
use yolo_hub_peer_rpc::{Client, RpcError};
use yolo_hub_peer_transport::ChannelAdmissionError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DiscoveryRefresh {
    Updated,
    DeferredLocalAdmission,
}

/// Only failures proven local and preceding RPC dispatch defer discovery.
/// A remote refusal, malformed reply or closed transport still retires the link.
pub(crate) async fn open(link: &Link, deadline: Instant) -> Result<Option<Client>> {
    let stream = match link.connection.open_channel(deadline).await {
        Ok(stream) => stream,
        Err(error)
            if error.downcast_ref::<ChannelAdmissionError>()
                == Some(&ChannelAdmissionError::AtCapacity) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    classify_connection(Client::connect(stream, deadline).await)
}

fn classify_connection(result: std::result::Result<Client, RpcError>) -> Result<Option<Client>> {
    match result {
        Ok(client) => Ok(Some(client)),
        Err(RpcError::Capacity) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/discovery_channel.rs"]
mod unit_tests;
