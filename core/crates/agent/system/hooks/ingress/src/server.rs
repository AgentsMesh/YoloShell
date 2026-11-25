use super::host::Context;
use anyhow::{Context as _, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use yolo_local_unix::LocalUnixStream;
use yolo_paths::namespace_lease::NamespaceLease;

const SOCKET_LIVE_ERROR: &str = "provider hook socket is already live";
const SOCKET_UNAVAILABLE: &str = "provider hook listener is unavailable";
const SOCKET_PARENT_MISSING: &str = "provider hook socket has no parent";
const SOCKET_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(250);

pub struct HookIngressServer {
    listener: Option<UnixListener>,
    publication: Option<yolo_paths::socket_activation::SocketPublication>,
    socket: PathBuf,
    clients: Arc<Semaphore>,
}

impl HookIngressServer {
    pub async fn bind(
        namespace: Arc<NamespaceLease>,
        socket: PathBuf,
        clients: usize,
    ) -> Result<Self> {
        let (listener, publication) = publish(&namespace, &socket).await?;
        Ok(Self {
            listener: Some(listener),
            publication: Some(publication),
            socket,
            clients: Arc::new(Semaphore::new(clients)),
        })
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    pub async fn accept(
        &self,
    ) -> std::io::Result<(LocalUnixStream, tokio::net::unix::SocketAddr)> {
        match self.listener.as_ref() {
            Some(listener) => listener
                .accept()
                .await
                .map(|(stream, address)| (LocalUnixStream::from(stream), address)),
            None => Err(std::io::Error::other(SOCKET_UNAVAILABLE)),
        }
    }

    pub async fn restart(&mut self, namespace: &NamespaceLease) -> Result<()> {
        self.listener.take();
        self.publication.take();
        let (listener, publication) = publish(namespace, &self.socket).await?;
        self.listener = Some(listener);
        self.publication = Some(publication);
        Ok(())
    }

    pub(crate) fn admit_client(&self) -> Option<OwnedSemaphorePermit> {
        Arc::clone(&self.clients).try_acquire_owned().ok()
    }

    pub fn into_pump(
        self,
        host: super::host::HookIngressHost,
        executable: Arc<super::executable::HookExecutable>,
    ) -> (super::pump::HookIngressPump, impl std::future::Future<Output = Result<()>>) {
        let context = Context { host, executable };
        super::pump::start(self, context)
    }
}

async fn publish(
    namespace: &NamespaceLease,
    path: &Path,
) -> Result<(
    UnixListener,
    yolo_paths::socket_activation::SocketPublication,
)> {
    let parent = path.parent().context(SOCKET_PARENT_MISSING)?;
    let directory = namespace.directory(parent)?;
    yolo_paths::socket_activation::publish(directory, path, SOCKET_PROBE_TIMEOUT, SOCKET_LIVE_ERROR)
        .await
}
