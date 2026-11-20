use crate::{PeerPins, PeerStream, constants::*};
use russh::{
    Channel,
    keys::{PrivateKey, PublicKey},
    server,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Semaphore, mpsc};
use yolo_hub_delegation::PeerIdentity;

pub(crate) struct ServerHandler {
    pub pins: PeerPins,
    pub authenticated: mpsc::Sender<PeerIdentity>,
    pub peer: Option<PeerIdentity>,
    pub incoming: mpsc::Sender<PeerStream>,
    pub capacity: Arc<Semaphore>,
    pub closed: Arc<AtomicBool>,
}
impl Drop for ServerHandler {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Release);
    }
}
impl server::Handler for ServerHandler {
    type Error = anyhow::Error;
    async fn auth_publickey_offered(
        &mut self,
        user: &str,
        key: &PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        Ok(if user == PEER_USER && self.pins.identify(key).is_some() {
            server::Auth::Accept
        } else {
            server::Auth::reject()
        })
    }
    async fn auth_publickey(
        &mut self,
        user: &str,
        key: &PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        if user != PEER_USER || self.peer.is_some() {
            return Ok(server::Auth::reject());
        }
        let Some(peer) = self.pins.identify(key) else {
            return Ok(server::Auth::reject());
        };
        if self.authenticated.try_send(peer).is_err() {
            return Ok(server::Auth::reject());
        }
        self.peer = Some(peer);
        Ok(server::Auth::Accept)
    }
    async fn channel_open_session(
        &mut self,
        channel: Channel<server::Msg>,
        reply: server::ChannelOpenHandle,
        _: &mut server::Session,
    ) -> Result<(), Self::Error> {
        if self.peer.is_none() {
            return Ok(());
        }
        let Ok(capacity) = self.capacity.clone().try_acquire_owned() else {
            return Ok(());
        };
        let Ok(queued) = self.incoming.clone().try_reserve_owned() else {
            return Ok(());
        };
        reply.accept().await;
        queued.send(PeerStream::new(channel.into_stream(), capacity));
        Ok(())
    }
}
pub(crate) fn config(key: Arc<PrivateKey>) -> Arc<server::Config> {
    Arc::new(server::Config {
        keys: vec![(*key).clone()],
        channel_buffer_size: SSH_CHANNEL_MESSAGES,
        event_buffer_size: SSH_SESSION_MESSAGES,
        keepalive_interval: Some(KEEPALIVE_INTERVAL),
        keepalive_max: MISSED_KEEPALIVES,
        auth_rejection_time: AUTH_REJECTION_DELAY,
        ..Default::default()
    })
}
