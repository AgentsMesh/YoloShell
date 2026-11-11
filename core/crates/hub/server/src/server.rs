use crate::{connection, constants::*};
use anyhow::{Result, ensure};
use std::{future::Future, sync::Arc};
use tokio::{net::UnixListener, task::JoinSet, time};
use yolo_agent_provider_contract::ProjectionPolicies;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_state::HubState;
use yolo_local_unix::LocalUnixStream;
use yolo_paths::socket_activation::{SocketPublication, publish};

pub struct HubServer {
    listener: UnixListener,
    publication: SocketPublication,
    state: Arc<HubState>,
    dispatcher: Arc<dyn Dispatcher>,
}

impl HubServer {
    pub async fn bind(state: HubState, backend: LocalBackend) -> Result<Self> {
        ensure!(state.hub_id() == backend.hub_id(), IDENTITY_MISMATCH);
        let directory = state.directory()?;
        let backend = backend.with_layouts(&directory)?;
        let backend = if backend.agents_enabled() {
            backend
        } else {
            backend.with_agents(
                directory.ensure_descendant(&directory.path().join(AGENT_DIRECTORY))?,
                ProjectionPolicies::none(),
            )?
        };
        Self::bind_dispatcher(Arc::new(state), Arc::new(backend)).await
    }

    pub async fn bind_dispatcher(
        state: Arc<HubState>,
        dispatcher: Arc<dyn Dispatcher>,
    ) -> Result<Self> {
        Self::bind_dispatcher_audited(state, dispatcher, None).await
    }

    pub async fn bind_dispatcher_audited(
        state: Arc<HubState>,
        dispatcher: Arc<dyn Dispatcher>,
        audit: Option<Arc<yolo_hub_audit::AuditLog>>,
    ) -> Result<Self> {
        ensure!(state.hub_id() == dispatcher.hub_id(), IDENTITY_MISMATCH);
        let dispatcher = match audit {
            Some(log) => Arc::new(yolo_hub_audit::AuditedDispatcher::new(
                dispatcher,
                log,
                AUDIT_LOCAL_PRINCIPAL,
            )),
            None => dispatcher,
        };
        let (listener, publication) = publish(
            state.socket_directory()?,
            &state.socket_path(),
            SOCKET_PROBE_TIMEOUT,
            LIVE_SOCKET,
        )
        .await?;
        state.verify()?;
        Ok(Self {
            listener,
            publication,
            state,
            dispatcher,
        })
    }

    /// Closing ingress never takes over or terminates underlying Worker/PTY custody.
    pub async fn serve_until(self, shutdown: impl Future<Output = ()>) -> Result<()> {
        let mut channels = JoinSet::new();
        let mut verify = time::interval(HEARTBEAT_INTERVAL);
        tokio::pin!(shutdown);
        let result = loop {
            tokio::select! {
                _ = &mut shutdown => break Ok(()),
                _ = verify.tick() => if let Err(error) = self.state.verify() { break Err(error); },
                joined = channels.join_next(), if !channels.is_empty() => {
                    if !matches!(joined, Some(Ok(Ok(())))) { tracing::debug!(CHANNEL_FAILED); }
                },
                accepted = self.listener.accept() => {
                    let (stream, _) = match accepted { Ok(value) => value, Err(error) => break Err(error.into()) };
                    if channels.len() >= MAX_CLIENT_CHANNELS { continue; }
                    if let Err(error) = self.state.verify() { break Err(error); }
                    let stream = LocalUnixStream::from(stream);
                    if !connection::same_user(&stream) { continue; }
                    channels.spawn(connection::serve(stream, self.dispatcher.clone(), self.state.clone()));
                },
            }
        };
        channels.abort_all();
        while channels.join_next().await.is_some() {}
        drop(self.listener);
        let cleanup = self.publication.withdraw();
        result.and(cleanup)
    }
}
