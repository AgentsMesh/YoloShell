use crate::{connection, constants::*};
use anyhow::{Result, ensure};
use std::{future::Future, path::PathBuf, sync::Arc};
use tokio::{net::UnixListener, sync::Semaphore, task::JoinSet};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_state::HubState;
use yolo_paths::socket_activation::{SocketPublication, publish};

pub struct PublicClientServer {
    state: Arc<HubState>,
    registry: AccessRegistry,
    dispatcher: Arc<dyn Dispatcher>,
    key: Arc<russh::keys::PrivateKey>,
    audit: Option<Arc<yolo_hub_audit::AuditLog>>,
    listener: UnixListener,
    publication: Option<SocketPublication>,
    socket: PathBuf,
}

impl PublicClientServer {
    pub async fn bind(
        state: Arc<HubState>,
        dispatcher: Arc<dyn Dispatcher>,
        registry: AccessRegistry,
    ) -> Result<Self> {
        Self::bind_audited(state, dispatcher, registry, None).await
    }

    pub async fn bind_audited(
        state: Arc<HubState>,
        dispatcher: Arc<dyn Dispatcher>,
        registry: AccessRegistry,
        audit: Option<Arc<yolo_hub_audit::AuditLog>>,
    ) -> Result<Self> {
        ensure!(
            state.hub_id() == dispatcher.hub_id()
                && state.hub_id() == registry.current()?.entry_hub,
            IDENTITY_MISMATCH
        );
        let directory = state.socket_directory()?;
        let socket = directory.path().join(PUBLIC_CLIENT_SOCKET);
        let key = Arc::new(state.transport_key()?);
        let (listener, publication) =
            publish(directory, &socket, SOCKET_PROBE_TIMEOUT, LIVE_SOCKET).await?;
        state.verify()?;
        Ok(Self {
            state,
            registry,
            dispatcher,
            key,
            audit,
            listener,
            publication: Some(publication),
            socket,
        })
    }

    pub fn socket_path(&self) -> &std::path::Path {
        &self.socket
    }

    pub async fn serve_until(mut self, shutdown: impl Future<Output = ()>) -> Result<()> {
        let mut connections = JoinSet::new();
        let capacity = Arc::new(Semaphore::new(MAX_ACTIVE_CHANNELS));
        let mut verify = tokio::time::interval(VERIFY_INTERVAL);
        tokio::pin!(shutdown);
        let result = loop {
            tokio::select! {
                _ = &mut shutdown => break Ok(()),
                _ = verify.tick() => {
                    if let Err(error) = self.state.verify().and_then(|()| self.registry.current().map(|_| ()).map_err(Into::into)) {
                        break Err(error);
                    }
                },
                completed = connections.join_next(), if !connections.is_empty() => {
                    if !matches!(completed, Some(Ok(Ok(())))) { tracing::debug!(CHANNEL_FAILED); }
                },
                accepted = self.listener.accept() => {
                    let (stream, _) = match accepted { Ok(value) => value, Err(error) => break Err(error.into()) };
                    if connections.len() >= MAX_CONNECTIONS { continue; }
                    if let Err(error) = self.state.verify() { break Err(error); }
                    connections.spawn(connection::serve(stream, self.key.clone(), self.registry.clone(), self.dispatcher.clone(), self.state.clone(), capacity.clone(), self.audit.clone()));
                },
            }
        };
        self.registry.close();
        connections.abort_all();
        while connections.join_next().await.is_some() {}
        if let Some(publication) = self.publication.take() {
            publication.withdraw()?;
        }
        result
    }
}

impl Drop for PublicClientServer {
    fn drop(&mut self) {
        self.registry.close();
    }
}
