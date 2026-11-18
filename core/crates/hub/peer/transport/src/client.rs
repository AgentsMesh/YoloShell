use crate::{PeerStream, constants::*, pins::matches_key};
use russh::{Channel, client};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Semaphore, mpsc};
use yolo_hub_delegation::PeerIdentity;

pub(crate) struct ClientHandler {
    pub peer: PeerIdentity,
    pub incoming: mpsc::Sender<PeerStream>,
    pub capacity: Arc<Semaphore>,
    pub closed: Arc<AtomicBool>,
}
impl Drop for ClientHandler {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Release);
    }
}
impl client::Handler for ClientHandler {
    type Error = anyhow::Error;
    async fn check_server_key(
        &mut self,
        key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(matches_key(self.peer, key))
    }
    async fn server_channel_open_session(
        &mut self,
        channel: Channel<client::Msg>,
        reply: client::ChannelOpenHandle,
        _: &mut client::Session,
    ) -> Result<(), Self::Error> {
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
pub(crate) fn config() -> Arc<client::Config> {
    Arc::new(client::Config {
        channel_buffer_size: SSH_CHANNEL_MESSAGES,
        keepalive_interval: Some(KEEPALIVE_INTERVAL),
        keepalive_max: MISSED_KEEPALIVES,
        ..Default::default()
    })
}
