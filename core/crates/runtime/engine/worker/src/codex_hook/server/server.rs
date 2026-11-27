use super::constants::*;
use super::hook_context::CodexHookContext;
use super::outcome::CodexHookHandleOutcome;
use super::router::CodexSessionRouter;
use crate::local_unix::LocalUnixStream;
use crate::worker::WorkerNamespace;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;

pub(in crate::worker) struct CodexHookServer {
    socket_directory: crate::paths::directory::PrivateDirectory,
    listener: Option<UnixListener>,
    publication: Option<crate::paths::socket_activation::SocketPublication>,
    path: PathBuf,
    router: Arc<CodexSessionRouter>,
    transaction: Arc<CodexHookContext>,
    clients: Arc<Semaphore>,
    settlements: super::blocking_custody::BlockingSettlementCustody,
    generation: super::cancellation::IngressCancellation,
}

impl CodexHookServer {
    fn finish_bind(
        socket_directory: crate::paths::directory::PrivateDirectory,
        path: PathBuf,
        listener: UnixListener,
        publication: crate::paths::socket_activation::SocketPublication,
        router: Arc<CodexSessionRouter>,
        transaction: Arc<CodexHookContext>,
        settlements: super::blocking_custody::BlockingSettlementCustody,
        generation: super::cancellation::IngressCancellation,
    ) -> Self {
        Self {
            socket_directory,
            listener: Some(listener),
            publication: Some(publication),
            path,
            router,
            transaction,
            clients: Arc::new(Semaphore::new(CODEX_MAX_CLIENTS)),
            settlements,
            generation,
        }
    }

    pub(in crate::worker) fn socket_path(&self) -> &Path {
        &self.path
    }

    pub(super) fn router(&self) -> Arc<CodexSessionRouter> {
        Arc::clone(&self.router)
    }

    pub(super) fn transaction(&self) -> Arc<CodexHookContext> {
        Arc::clone(&self.transaction)
    }

    pub(super) fn settlements(&self) -> super::blocking_custody::BlockingSettlementCustody {
        self.settlements.clone()
    }

    pub(super) fn generation(&self) -> super::cancellation::IngressCancellation {
        self.generation.clone()
    }

    pub(in crate::worker) async fn accept(
        &self,
    ) -> std::io::Result<(LocalUnixStream, tokio::net::unix::SocketAddr)> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| std::io::Error::other(CODEX_LISTENER_ERROR))?;
        listener
            .accept()
            .await
            .map(|(stream, address)| (LocalUnixStream::from(stream), address))
    }

    pub(in crate::worker) async fn handle_stream(
        &self,
        stream: LocalUnixStream,
    ) -> Result<CodexHookHandleOutcome> {
        let _permit = self.admit_client()?;
        super::routed_handler::handle(
            stream,
            self.router(),
            self.transaction(),
            self.settlements(),
            self.generation(),
        )
        .await
    }

    pub(super) fn try_admit_client(&self) -> Option<OwnedSemaphorePermit> {
        Arc::clone(&self.clients).try_acquire_owned().ok()
    }

    fn admit_client(&self) -> Result<OwnedSemaphorePermit> {
        Arc::clone(&self.clients)
            .try_acquire_owned()
            .context(CODEX_CLIENT_LIMIT_ERROR)
    }
}

include!("server_bind.rs");
include!("server_restart.rs");
include!("server_socket.rs");
include!("server_native_namespace.rs");
