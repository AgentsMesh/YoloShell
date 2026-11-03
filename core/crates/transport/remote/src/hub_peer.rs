use super::{ChannelIdentity, RemoteError};
use std::{net::SocketAddr, num::NonZeroU16};
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;

const INVALID_ENDPOINT: &str = "Hub peer endpoint must be a nonzero unscoped loopback address";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HubPeerEndpoint {
    address: std::net::IpAddr,
    port: NonZeroU16,
}

impl HubPeerEndpoint {
    pub fn new(address: SocketAddr) -> Result<Self, RemoteError> {
        let scoped = matches!(address, SocketAddr::V6(value) if value.scope_id() != 0 || value.flowinfo() != 0);
        if !address.ip().is_loopback() || scoped {
            return Err(RemoteError::InvalidEndpoint(INVALID_ENDPOINT.into()));
        }
        let port = NonZeroU16::new(address.port())
            .ok_or_else(|| RemoteError::InvalidEndpoint(INVALID_ENDPOINT.into()))?;
        Ok(Self {
            address: address.ip(),
            port,
        })
    }
    pub fn address(self) -> SocketAddr {
        SocketAddr::new(self.address, self.port.get())
    }
}

/// A coordinator constructs this after verifying its saved enrollment receipt.
/// It authorizes one pinned Hub target on one exact outer SSH connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HubPeerRequest {
    connection: Uuid,
    endpoint: HubPeerEndpoint,
    peer: PeerIdentity,
}

impl HubPeerRequest {
    pub fn new(
        connection: Uuid,
        endpoint: HubPeerEndpoint,
        peer: PeerIdentity,
    ) -> Result<Self, RemoteError> {
        if connection.is_nil() {
            return Err(RemoteError::ChannelMismatch);
        }
        Ok(Self {
            connection,
            endpoint,
            peer,
        })
    }
    pub fn connection_id(self) -> Uuid {
        self.connection
    }
    pub fn endpoint(self) -> HubPeerEndpoint {
        self.endpoint
    }
    pub fn peer(self) -> PeerIdentity {
        self.peer
    }
}

pub struct OpenedHubPeerChannel {
    pub identity: ChannelIdentity,
    /// The consumer must still perform the inner mutually pinned Hub handshake.
    pub peer: PeerIdentity,
    pub stream: super::russh_backend::HubPeerStream,
}
