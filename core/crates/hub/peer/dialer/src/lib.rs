//! Transport injection only. Federation always performs its own mutually pinned handshake.
mod constants;
mod tcp;
pub use tcp::TcpPeerDialer;

use anyhow::{Context, Result, ensure};
use constants::*;
use std::{future::Future, pin::Pin};
use tokio::time::{Instant, timeout_at};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::PeerAddress;
use yolo_hub_peer_transport::PeerIo;

pub type PeerDialFuture<'a> = Pin<Box<dyn Future<Output = Result<PeerIo>> + Send + 'a>>;

pub trait PeerDialer: Send + Sync {
    /// Opens transport for this complete configured identity and address. The factory
    /// receives no local private key and grants no peer authority. Dropping this future
    /// must cancel pending work while retaining responsibility for resources already opened.
    /// Returned PeerIo owns a nonblocking close action scoped to this one transport.
    fn dial<'a>(
        &'a self,
        peer: PeerIdentity,
        address: &'a PeerAddress,
        deadline: Instant,
    ) -> PeerDialFuture<'a>;
}

/// Enforces one absolute opening budget even when an injected factory ignores its deadline.
/// Failed custom transport selection never falls back to the default TCP implementation.
pub async fn dial_peer(
    dialer: &dyn PeerDialer,
    peer: PeerIdentity,
    address: &PeerAddress,
    deadline: Instant,
) -> Result<PeerIo> {
    require_budget(deadline)?;
    address.validate()?;
    let transport = timeout_at(deadline, dialer.dial(peer, address, deadline))
        .await
        .context(DEADLINE_EXPIRED)??;
    require_budget(deadline)?;
    Ok(transport)
}

pub(crate) fn require_budget(deadline: Instant) -> Result<()> {
    ensure!(Instant::now() < deadline, DEADLINE_EXPIRED);
    Ok(())
}
