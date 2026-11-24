use super::{
    audit::{Failure, Shared},
    constants::*,
};
use crate::hub::support::deadline;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    net::UnixListener,
    sync::oneshot,
    task::{JoinHandle, JoinSet},
};
use yolo_hub_terminal_protocol::CreateShellResult;
use yolo_tui_creation_journal::CreationIntent;
#[path = "tui_creation_recovery_proxy_connection.rs"]
mod connection;

#[derive(Clone, Copy, Debug)]
enum Checkpoint {
    CreatedHeld,
    OldConnectionClosed,
    Finalized,
}

pub struct Proxy {
    pub socket: PathBuf,
    shared: Arc<Shared>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
    _root: tempfile::TempDir,
}

impl Proxy {
    pub fn start(origin: &Path, workspace: &Path) -> Self {
        let root = tempfile::tempdir_in(crate::hub::support::SOCKET_TEST_ROOT).unwrap();
        let socket = root.path().canonicalize().unwrap().join(PROXY_SOCKET);
        let listener = UnixListener::bind(&socket).unwrap();
        let shared = Arc::new(Shared::default());
        let state = shared.clone();
        let origin = origin.to_path_buf();
        let workspace = workspace.to_path_buf();
        let (stop, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut tasks = JoinSet::new();
            loop {
                tokio::select! {
                    biased;
                    _ = &mut stopped => break,
                    result = tasks.join_next(), if !tasks.is_empty() => match result {
                        Some(Ok(Err(error))) => state.fail(error),
                        Some(Err(_)) => state.fail(Failure::Task),
                        _ => {}
                    },
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else { state.fail(Failure::Accept); break; };
                        if tasks.len() >= MAX_PROXY_CONNECTIONS { state.fail(Failure::Capacity); break; }
                        tasks.spawn(connection::run(stream, origin.clone(), workspace.clone(), state.clone()));
                    }
                }
            }
            tasks.abort_all();
            while let Some(result) = tasks.join_next().await {
                match result {
                    Ok(Err(error)) => state.fail(error),
                    Err(error) if !error.is_cancelled() => state.fail(Failure::Task),
                    _ => {}
                }
            }
        });
        Self {
            socket,
            shared,
            stop: Some(stop),
            task: Some(task),
            _root: root,
        }
    }

    pub async fn lost_creation(&self) -> (CreationIntent, CreateShellResult) {
        self.wait(Checkpoint::CreatedHeld, |audit| audit.creation.is_some())
            .await;
        let audit = self.shared.audit.lock().unwrap();
        (audit.intent.clone().unwrap(), audit.creation.unwrap())
    }

    pub async fn old_connection_closed(&self) {
        self.wait(Checkpoint::OldConnectionClosed, |audit| audit.held_closed)
            .await;
    }

    pub async fn finalized(&self) {
        self.wait(Checkpoint::Finalized, |audit| audit.finalized)
            .await;
    }

    async fn wait(&self, checkpoint: Checkpoint, ready: impl Fn(&super::audit::Audit) -> bool) {
        tokio::time::timeout_at(deadline(), async {
            loop {
                let changed = self.shared.changed.notified();
                {
                    let audit = self.shared.audit.lock().unwrap();
                    assert!(
                        audit.failure.is_none(),
                        "creation recovery proxy failed: {:?}",
                        audit.failure
                    );
                    if ready(&audit) {
                        return;
                    }
                }
                changed.await;
            }
        })
        .await
        .unwrap_or_else(|_| {
            let audit = self.shared.audit.lock().unwrap();
            panic!("creation recovery proxy deadline: checkpoint={checkpoint:?} creates={} queries={} finalizes={} held_closed={} failure={:?}",
                audit.creates, audit.queries, audit.finalizes, audit.held_closed, audit.failure);
        });
    }

    pub async fn stop(mut self) {
        self.stop.take().unwrap().send(()).unwrap();
        tokio::time::timeout_at(deadline(), self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        self.shared.audit.lock().unwrap().verify();
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.stop.take();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
