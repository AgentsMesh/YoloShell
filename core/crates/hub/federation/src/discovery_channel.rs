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

use crate::lifetime::OwnedGeneration;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_peer_protocol::{PeerReply, PeerRequest};
use yolo_hub_peer_rpc::{Handler, Response as RpcResponse, RpcFuture};

pub(crate) struct PeerHandler {
    pub(crate) peer: PeerIdentity,
    pub(crate) local_grant: yolo_hub_federation_directory::LocalGrant,
    pub(crate) directory_watch: yolo_hub_federation_directory::PeerDirectoryWatch,
    pub(crate) generation: OwnedGeneration,
}
impl Handler for PeerHandler {
    fn watch_directory(&self, request: PeerRequest) -> RpcFuture<'_, RpcResponse> {
        Box::pin(async move {
            let node = self.generation.node();
            node.require_open().map_err(error)?;
            self.directory_watch.before(&node.changes, &node.config, node.hub_id(), self.peer.hub(), &request)
                .await.map_err(error)?;
            self.request(request).await
        })
    }

    fn request(&self, request: PeerRequest) -> RpcFuture<'_, RpcResponse> {
        Box::pin(async move {
            let node = self.generation.node();
            match request {
                PeerRequest::Discover {
                    request_id,
                    version,
                    allowed_origins,
                } => {
                    let reply = node
                        .directory(
                            self.peer,
                            request_id,
                            version,
                            allowed_origins,
                            &self.local_grant,
                        )
                        .await
                        .map_err(error)?;
                    Ok(RpcResponse { reply, updates: None })
                }
                PeerRequest::Execute { call } => {
                    node.require_open().map_err(error)?;
                    node.clone().execute(self.peer, call).await.map_err(error)
                }
            }
        })
    }
}
fn error(_: anyhow::Error) -> RpcError { RpcError::Invalid }
