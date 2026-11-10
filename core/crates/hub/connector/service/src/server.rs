use crate::{connection, constants::*};
use anyhow::{Result, ensure};
use std::{future::Future, path::PathBuf, sync::Arc};
use tokio::{net::UnixListener, sync::oneshot, task::JoinSet, time};
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_state::HubState;
use yolo_local_unix::LocalUnixStream;
use yolo_paths::socket_activation::{SocketPublication, publish};

pub struct ConnectorServer {
    listener: UnixListener,
    publication: SocketPublication,
    socket: PathBuf,
    state: Arc<HubState>,
    backend: LocalBackend,
}
impl ConnectorServer {
    pub async fn bind(state: Arc<HubState>, backend: LocalBackend) -> Result<Self> {
        ensure!(state.hub_id() == backend.hub_id(), INVALID_BACKEND);
        backend.list_connectors().await?;
        let directory = state.socket_directory()?;
        let socket = directory.path().join(CONNECTOR_SOCKET_FILE);
        let (listener, publication) =
            publish(directory, &socket, PROBE_TIMEOUT, LIVE_SOCKET).await?;
        state.verify()?;
        Ok(Self {
            listener,
            publication,
            socket,
            state,
            backend,
        })
    }
    pub fn socket_path(&self) -> PathBuf {
        self.socket.clone()
    }

    pub async fn serve_until(self, shutdown: impl Future<Output = ()>) -> Result<()> {
        let (cancel, cancelled) = oneshot::channel();
        let mut serving = tokio::spawn(self.serve_owned(cancelled));
        tokio::pin!(shutdown);
        // Cancelling this wrapper drops cancel; the owned task still retires its
        // channels and registry generations before relinquishing the socket.
        tokio::select! {
            result = &mut serving => result?,
            _ = &mut shutdown => { drop(cancel); serving.await? }
        }
    }

    async fn serve_owned(self, shutdown: oneshot::Receiver<()>) -> Result<()> {
        let mut channels = JoinSet::new();
        let mut verify = time::interval(VERIFY_INTERVAL);
        tokio::pin!(shutdown);
        let result = loop {
            tokio::select! {
                _ = &mut shutdown => break Ok(()),
                _ = verify.tick() => if let Err(error) = self.state.verify() { break Err(error); },
                _ = channels.join_next(), if !channels.is_empty() => {},
                accepted = self.listener.accept() => {
                    let (stream, _) = match accepted { Ok(value) => value, Err(error) => break Err(error.into()) };
                    if channels.len() >= MAX_CLIENTS { continue; }
                    let stream = LocalUnixStream::from(stream);
                    if !connection::same_user(&stream) { continue; }
                    channels.spawn(connection::serve(stream, self.state.clone(), self.backend.clone()));
                },
            }
        };
        channels.abort_all();
        while channels.join_next().await.is_some() {}
        // Aborted connection futures cannot run async teardown; retire all their
        // generations here before this listener finishes its shutdown contract.
        let disconnected = self.backend.disconnect_all_connectors().await;
        // Keep the listener published until cleanup completes; a replacement
        // service must not activate publishers before this owner's final sweep.
        drop(self.listener);
        let withdrawn = self.publication.withdraw();
        result?;
        disconnected?;
        withdrawn
    }
}
