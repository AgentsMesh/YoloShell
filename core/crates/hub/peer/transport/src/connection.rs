use crate::{
    ChannelAdmissionError, PeerStream, client::ClientHandler, constants::*, io_custody::IoCustody,
    server::ServerHandler,
};
use anyhow::{Context, Result, ensure};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    sync::{Mutex, Semaphore},
    time::{Instant, timeout_at},
};
use yolo_hub_delegation::PeerIdentity;

pub struct PeerConnection {
    /// Present for peer links; enrollment connections authenticate through an
    /// invite and never become federation links.
    pub(crate) peer: Option<PeerIdentity>,
    pub(crate) role: Role,
    pub(crate) transport: IoCustody,
    pub(crate) capacity: Arc<Semaphore>,
    pub(crate) closed: Arc<AtomicBool>,
}
pub(crate) enum Role {
    Client(Mutex<Option<russh::client::Handle<ClientHandler>>>),
    Server {
        handle: russh::server::Handle,
        running: Mutex<Option<russh::server::RunningSession<ServerHandler>>>,
    },
}
impl PeerConnection {
    pub fn peer(&self) -> PeerIdentity {
        self.peer.expect("enrollment connections have no authenticated peer")
    }
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
    pub async fn open_channel(&self, deadline: Instant) -> Result<PeerStream> {
        ensure!(!self.is_closed(), CLOSED);
        let permit = self
            .capacity
            .clone()
            .try_acquire_owned()
            .map_err(ChannelAdmissionError::from)?;
        let stream = match &self.role {
            Role::Client(handle) => {
                let handle = timeout_at(deadline, handle.lock()).await?;
                let handle = handle.as_ref().context(CLOSED)?;
                PeerStream::new(
                    timeout_at(deadline, handle.channel_open_session())
                        .await??
                        .into_stream(),
                    permit,
                )
            }
            Role::Server { handle, .. } => PeerStream::new(
                timeout_at(deadline, handle.channel_open_session())
                    .await??
                    .into_stream(),
                permit,
            ),
        };
        Ok(stream)
    }
    pub async fn close(&self, deadline: Instant) -> Result<()> {
        self.closed.store(true, Ordering::Release);
        self.transport.close();
        match &self.role {
            Role::Client(handle) => {
                if let Some(handle) = timeout_at(deadline, handle.lock()).await?.take() {
                    let _ = timeout_at(deadline, handle).await?;
                }
            }
            Role::Server { running, .. } => {
                if let Some(running) = timeout_at(deadline, running.lock()).await?.take() {
                    let _ = timeout_at(deadline, running).await?;
                }
            }
        }
        Ok(())
    }
}
