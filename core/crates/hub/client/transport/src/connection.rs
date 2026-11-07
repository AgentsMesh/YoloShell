use crate::{
    ClientStream, HubPin,
    client::ClientHandler,
    constants::*,
    custody::{IoCustody, OpeningGuard},
    server::ServerHandler,
};
use anyhow::{Context, Result, ensure};
use std::sync::Arc;
use tokio::{
    sync::{Mutex, Semaphore},
    task::JoinHandle,
    time::{Instant, timeout_at},
};
use uuid::Uuid;
use yolo_hub_client_access_registry::ClientAdmission;

pub struct ClientConnection {
    pub(crate) pin: HubPin,
    pub(crate) handle: Mutex<Option<russh::client::Handle<ClientHandler>>>,
    pub(crate) transport: IoCustody,
    pub(crate) capacity: Arc<Semaphore>,
}
impl ClientConnection {
    pub fn hub(&self) -> HubPin {
        self.pin
    }
    pub fn is_closed(&self) -> bool {
        self.transport.0.is_closed()
    }
    pub async fn closed(&self) {
        self.transport.0.closed().await;
    }
    pub async fn open_channel(&self, deadline: Instant) -> Result<ClientStream> {
        ensure!(!self.is_closed(), CLOSED);
        let permit = self
            .capacity
            .clone()
            .try_acquire_owned()
            .context(CHANNEL_LIMIT)?;
        let handle = timeout_at(deadline, self.handle.lock()).await?;
        let handle = handle.as_ref().context(CLOSED)?;
        let mut custody = OpeningGuard(Some(self.transport.0.clone()));
        let result = timeout_at(deadline, handle.channel_open_session()).await?;
        // A completed rejection has no orphaned channel and preserves siblings.
        custody.0.take();
        Ok(ClientStream::new(result?.into_stream(), permit))
    }
    pub async fn close(&self, deadline: Instant) -> Result<()> {
        self.transport.0.close();
        if let Some(handle) = timeout_at(deadline, self.handle.lock()).await?.take() {
            let _ = timeout_at(deadline, handle).await?;
        }
        Ok(())
    }
}

/// Holds accepted client custody; deliberately exposes no reverse-channel API.
pub struct AcceptedConnection {
    pub(crate) admission: ClientAdmission,
    pub(crate) connection_id: Uuid,
    pub(crate) transport: IoCustody,
    pub(crate) running: Mutex<Option<russh::server::RunningSession<ServerHandler>>>,
    pub(crate) revocation: JoinHandle<()>,
}
impl AcceptedConnection {
    pub fn admission(&self) -> &ClientAdmission {
        &self.admission
    }
    pub fn connection_id(&self) -> Uuid {
        self.connection_id
    }
    pub fn is_closed(&self) -> bool {
        self.transport.0.is_closed()
    }
    pub async fn closed(&self) {
        self.transport.0.closed().await;
    }
    pub async fn close(&self, deadline: Instant) -> Result<()> {
        self.transport.0.close();
        self.revocation.abort();
        if let Some(running) = timeout_at(deadline, self.running.lock()).await?.take() {
            let _ = timeout_at(deadline, running).await?;
        }
        Ok(())
    }
}
impl Drop for AcceptedConnection {
    fn drop(&mut self) {
        self.revocation.abort();
    }
}
