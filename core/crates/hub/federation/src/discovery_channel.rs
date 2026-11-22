use anyhow::Result;
use yolo_hub_peer_rpc::{Received, RpcError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DiscoveryRefresh {
    Updated,
    DeferredLocalAdmission,
}

/// Only a typed local capacity refusal defers discovery. A remote refusal,
/// malformed reply or closed transport still retires the link.
pub(crate) fn classify(
    result: std::result::Result<Received, RpcError>,
) -> Result<Option<Received>> {
    match result {
        Ok(response) => Ok(Some(response)),
        Err(RpcError::Capacity) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/discovery_channel.rs"]
mod unit_tests;
