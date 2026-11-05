use crate::{Federation, constants::*};
use anyhow::{Result, ensure};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{sync::watch, time::interval};
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use yolo_hub_peer_rpc::Client;

/// One outbound, directed peer link: this Hub is the RPC client and the peer
/// is the provider. Calls and discovery share the connection.
pub(crate) struct Link {
    pub id: Uuid,
    pub peer: PeerIdentity,
    pub client: Client,
    closed: AtomicBool,
}

impl Link {
    pub(crate) fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire) || self.client.is_closed()
    }
    fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }
}

impl Federation {
    pub(crate) async fn register(&self, peer: PeerIdentity, client: Client) -> Result<Arc<Link>> {
        self.state.verify()?;
        let mut connections = self.connections.write().await;
        ensure!(!self.closing.load(Ordering::Acquire), CLOSED);
        ensure!(
            self.config
                .peer(peer.hub())
                .is_some_and(|configured| configured.identity == peer),
            INVALID_PEER
        );
        let old = connections.get(&peer.hub()).cloned();
        let link = Arc::new(Link {
            id: Uuid::new_v4(),
            peer,
            client,
            closed: AtomicBool::new(false),
        });
        self.book.lock().await.connect(peer, link.id)?;
        connections.insert(peer.hub(), link.clone());
        drop(connections);
        if let Some(old) = old {
            old.close();
            let _ = self.book.lock().await.disconnect(peer.hub(), old.id);
        }
        Ok(link)
    }
    pub(crate) async fn retire(&self, link: &Link) {
        link.close();
        let peer = link.peer.hub();
        let mut connections = self.connections.write().await;
        if connections
            .get(&peer)
            .is_some_and(|current| current.id == link.id)
        {
            connections.remove(&peer);
            let _ = self.book.lock().await.disconnect(peer, link.id);
        }
    }
    pub(crate) async fn current(&self, peer: HubId) -> Option<Arc<Link>> {
        self.connections
            .read()
            .await
            .get(&peer)
            .filter(|link| !link.is_closed())
            .cloned()
    }

    pub(crate) async fn serve_link(
        self: Arc<Self>,
        link: Arc<Link>,
        mut stop: watch::Receiver<bool>,
    ) -> Result<()> {
        let mut tick = interval(POLL_INTERVAL);
        let consume = self
            .config
            .peer(link.peer.hub())
            .is_some_and(|peer| peer.consume);
        loop {
            tokio::select! {
                _ = stop.changed() => break,
                _ = tick.tick() => {
                    if self.is_closing() || link.is_closed() { break; }
                    if !consume { continue; }
                    // Local admission pressure defers only this refresh; it does not
                    // invalidate the authenticated link or extend any routing proof.
                    if let Err(error) = self.discover(&link).await {
                        tracing::debug!(peer = %link.peer.hub(), %error, "Hub peer discovery ended the link");
                        break;
                    }
                }
            }
        }
        self.retire(&link).await;
        Ok(())
    }
}
