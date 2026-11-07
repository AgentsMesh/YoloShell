use super::constants::*;
use super::native_environment::ClaudeHookEnvironment;
use crate::local_unix::LocalUnixStream;
use crate::worker::{WorkerNamespace, WorkerState, supervision};
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;
use uuid::Uuid;

pub(in crate::worker) struct ClaudeHookServer {
    listener: Option<UnixListener>,
    publication: Option<crate::paths::socket_activation::SocketPublication>,
    environment: ClaudeHookEnvironment,
    clients: Arc<Semaphore>,
}

impl ClaudeHookServer {
    pub(in crate::worker) async fn bind(
        namespace: &WorkerNamespace,
        paths: &crate::paths::Paths,
        terminal_id: Uuid,
        hook_executable: std::path::PathBuf,
    ) -> Result<Self> {
        let environment =
            ClaudeHookEnvironment::new(namespace, paths, terminal_id, hook_executable)?;
        let (listener, publication) = publish(namespace, environment.socket_path()).await?;
        Ok(Self {
            listener: Some(listener),
            publication: Some(publication),
            environment,
            clients: Arc::new(Semaphore::new(MAX_HOOK_CLIENTS)),
        })
    }

    #[cfg(test)]
    pub(in crate::worker) async fn bind_test_source(
        namespace: &WorkerNamespace,
        paths: &crate::paths::Paths,
        terminal_id: Uuid,
        _settings_directory: std::path::PathBuf,
        source: &std::path::Path,
    ) -> Result<Self> {
        Self::bind(namespace, paths, terminal_id, source.canonicalize()?).await
    }

    pub(in crate::worker) fn environment(&self) -> &ClaudeHookEnvironment {
        &self.environment
    }

    pub(in crate::worker) async fn accept(
        &self,
    ) -> std::io::Result<(LocalUnixStream, tokio::net::unix::SocketAddr)> {
        match self.listener.as_ref() {
            Some(listener) => listener
                .accept()
                .await
                .map(|(stream, address)| (LocalUnixStream::from(stream), address)),
            None => Err(std::io::Error::other(SOCKET_LISTENER_UNAVAILABLE)),
        }
    }

    pub(in crate::worker) async fn restart(&mut self, namespace: &WorkerNamespace) -> Result<()> {
        self.listener.take();
        self.publication.take();
        let (listener, publication) = publish(namespace, self.environment.socket_path()).await?;
        self.listener = Some(listener);
        self.publication = Some(publication);
        Ok(())
    }

    fn admit_client(&self) -> Option<OwnedSemaphorePermit> {
        Arc::clone(&self.clients).try_acquire_owned().ok()
    }
}

pub(in crate::worker) fn handle_accept_result(
    accepted: std::io::Result<(LocalUnixStream, tokio::net::unix::SocketAddr)>,
    tasks: &mut JoinSet<Option<supervision::TaskFailure>>,
    state: &Arc<WorkerState>,
    server: &ClaudeHookServer,
    shell: crate::process::ProcessIdentity,
    runtime_active: bool,
    failure_count: &mut u32,
) -> Option<std::time::Duration> {
    match accepted {
        Ok((stream, _)) => {
            *failure_count = 0;
            let permit = server.admit_client()?;
            let state = Arc::clone(state);
            let environment = server.environment.clone();
            supervision::spawn_task(tasks, HOOK_CONNECTION_TASK, async move {
                let _permit = permit;
                super::handler::handle(stream, state, shell, environment, runtime_active).await
            });
            None
        }
        Err(error) => Some(handle_listener_failure(error.into(), failure_count)),
    }
}

pub(in crate::worker) fn handle_listener_failure(
    error: anyhow::Error,
    failure_count: &mut u32,
) -> std::time::Duration {
    *failure_count = failure_count.saturating_add(ACCEPT_FAILURE_INCREMENT);
    let failure = supervision::TaskFailure::message(HOOK_CONNECTION_TASK, error.to_string());
    supervision::report_restart(
        &failure,
        supervision::RestartPolicy::default(),
        *failure_count,
    )
}

async fn publish(
    namespace: &WorkerNamespace,
    path: &std::path::Path,
) -> Result<(
    UnixListener,
    crate::paths::socket_activation::SocketPublication,
)> {
    let parent = path.parent().context(SOCKET_PARENT_MISSING)?;
    let directory = namespace.lease().directory(parent)?;
    crate::paths::socket_activation::publish(
        directory,
        path,
        SOCKET_PROBE_TIMEOUT,
        SOCKET_LIVE_ERROR,
    )
    .await
    .context(SOCKET_BIND_CONTEXT)
}
