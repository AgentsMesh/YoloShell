use super::{ChannelIdentity, RemoteError};
use std::path::{Component, PathBuf};
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;

const INVALID_ENDPOINT: &str = "Hub peer endpoint must be an absolute canonical Unix socket path";
const MAX_SOCKET_PATH_BYTES: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HubPeerEndpoint {
    path: PathBuf,
}

impl HubPeerEndpoint {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, RemoteError> {
        let path = path.into();
        let encoded = path.as_os_str().as_encoded_bytes();
        let raw = path
            .to_str()
            .ok_or_else(|| RemoteError::InvalidEndpoint(INVALID_ENDPOINT.into()))?;
        if !path.is_absolute()
            || raw.split('/').any(|part| part == "." || part == "..")
            || !(1..=MAX_SOCKET_PATH_BYTES).contains(&encoded.len())
            || encoded.iter().any(|byte| byte.is_ascii_control())
            || !path
                .components()
                .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
        {
            return Err(RemoteError::InvalidEndpoint(INVALID_ENDPOINT.into()));
        }
        Ok(Self { path })
    }
    pub fn into_path(self) -> PathBuf {
        self.path
    }
}

/// A coordinator constructs this after verifying its saved enrollment receipt.
/// It authorizes one pinned Hub target on one exact outer SSH connection.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub fn connection_id(&self) -> Uuid {
        self.connection
    }
    pub fn endpoint(&self) -> &HubPeerEndpoint {
        &self.endpoint
    }
    pub fn peer(&self) -> PeerIdentity {
        self.peer
    }
}

pub struct OpenedHubPeerChannel {
    pub identity: ChannelIdentity,
    /// The consumer must still perform the peer link identity handshake.
    pub peer: PeerIdentity,
    pub stream: super::russh_backend::HubPeerStream,
}
