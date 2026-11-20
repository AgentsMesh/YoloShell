use crate::{Federation, constants::*};
use anyhow::{Result, ensure};
use std::sync::{Arc, atomic::Ordering};
use tokio::{
    sync::{mpsc, watch},
    task::JoinSet,
    time::interval,
};
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_peer_transport::{PeerConnection, PeerStream};

pub(crate) struct Link {
    pub id: Uuid,
    pub initiator: HubId,
    pub connection: Arc<PeerConnection>,
}
impl Federation {
    pub(crate) async fn register(
        &self,
        connection: PeerConnection,
        initiator: HubId,
    ) -> Result<Arc<Link>> {
        self.state.verify()?;
        let peer = connection.peer();
        let mut connections = self.connections.write().await;
        ensure!(!self.closing.load(Ordering::Acquire), CLOSED);
        ensure!(
            self.config
                .peer(peer.hub())
                .is_some_and(|configured| configured.identity == peer),
            INVALID_PEER
        );
        let old = connections.get(&peer.hub()).cloned();
        if let Some(old) = &old {
            ensure!(
                old.connection.is_closed() || initiator < old.initiator,
                DUPLICATE_LINK
            );
        }
        let link = Arc::new(Link {
            id: Uuid::new_v4(),
            initiator,
            connection: Arc::new(connection),
        });
        self.book.lock().await.connect(peer, link.id)?;
        connections.insert(peer.hub(), link.clone());
        drop(connections);
        if let Some(old) = old {
            let _ = old
                .connection
                .close(tokio::time::Instant::now() + STOP_TIMEOUT)
                .await;
        }
        Ok(link)
    }
    pub(crate) async fn retire(&self, link: &Link) {
        let peer = link.connection.peer().hub();
        let mut connections = self.connections.write().await;
        if connections
            .get(&peer)
            .is_some_and(|current| current.id == link.id)
        {
            connections.remove(&peer);
            let _ = self.book.lock().await.disconnect(peer, link.id);
        }
        drop(connections);
        let _ = link
            .connection
            .close(tokio::time::Instant::now() + STOP_TIMEOUT)
            .await;
    }
    pub(crate) async fn current(&self, peer: HubId) -> Option<Arc<Link>> {
        self.connections
            .read()
            .await
            .get(&peer)
            .filter(|link| !link.connection.is_closed())
            .cloned()
    }

    pub(crate) async fn serve_link(
        self: Arc<Self>,
        link: Arc<Link>,
        mut incoming: mpsc::Receiver<PeerStream>,
        mut stop: watch::Receiver<bool>,
    ) -> Result<()> {
        let mut tasks = JoinSet::new();
        let mut tick = interval(POLL_INTERVAL);
        let peer = link.connection.peer();
        let consume = self
            .config
            .peer(peer.hub())
            .is_some_and(|peer| peer.consume);
        let polling = async {
            if !consume {
                return std::future::pending::<Result<()>>().await;
            }
            let mut tick = interval(POLL_INTERVAL);
            loop {
                tick.tick().await;
                // Local admission pressure defers only this refresh; it does not
                // invalidate the authenticated link or extend any routing proof.
                self.discover(&link).await?;
            }
        };
        tokio::pin!(polling);
        loop {
            tokio::select! {
                _ = stop.changed() => break,
                _ = &mut polling => break,
                _ = tasks.join_next(), if !tasks.is_empty() => {},
                _ = tick.tick() => {
                    if self.is_closing() || link.connection.is_closed() { break; }
                }
                incoming = incoming.recv() => match incoming {
                    None => break,
                    Some(stream) => { let node = self.clone(); tasks.spawn(async move { node.handle(peer, stream).await }); }
                }
            }
        }
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        self.retire(&link).await;
        Ok(())
    }
}
