//! One SSH-streamlocal byte stream carrying one peer RPC link.
mod frame;
mod handshake;
mod message;

pub use yolo_hub_peer_io::Duplex;
use tokio::time::Instant;
use yolo_hub_delegation::{PeerIdentity, Signer};
use yolo_hub_peer_rpc::{Client, RpcError};

#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("peer link handshake frame is invalid")]
    Frame,
    #[error("peer link handshake version is unsupported")]
    Version,
    #[error("peer link handshake role is invalid")]
    Role,
    #[error("peer link identity is not the expected peer")]
    Identity,
    #[error("peer link signature is invalid")]
    Signature,
    #[error("peer link handshake timed out")]
    Deadline,
    #[error(transparent)]
    Rpc(#[from] RpcError),
}

/// Runs the client hello and returns the multiplexed RPC client for this link.
pub async fn connect<S: Duplex + 'static>(
    mut stream: S,
    local: PeerIdentity,
    signer: &impl Signer,
    expected_remote: PeerIdentity,
    deadline: Instant,
) -> Result<Client, LinkError> {
    handshake::client(&mut stream, local, signer, expected_remote, deadline).await?;
    Client::connect(stream, deadline).await.map_err(Into::into)
}

/// Verifies the client hello against the configured peers and returns the
/// authenticated identity plus the stream for `yolo_hub_peer_rpc::serve`.
pub async fn accept<S: Duplex + 'static>(
    mut stream: S,
    local: PeerIdentity,
    signer: &impl Signer,
    allowed_clients: &[PeerIdentity],
    deadline: Instant,
) -> Result<(PeerIdentity, S), LinkError> {
    let peer = handshake::server(&mut stream, local, signer, allowed_clients, deadline).await?;
    Ok((peer, stream))
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
