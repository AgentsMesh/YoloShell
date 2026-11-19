mod backend;
mod constants;
mod faulty;
mod prepared;
pub mod probes;
pub use backend::Backend;
pub use constants::*;
pub use faulty::FaultyBackend;
pub use prepared::Prepared;
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{sync::oneshot, task::JoinHandle, time::Instant};
use yolo_hub_client_access_protocol::*;
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{ClientConnection, ClientIo, HubPin};
use yolo_hub_identity::HubId;
use yolo_hub_public_client_server::PublicClientServer;
use yolo_hub_routing::Right;
use yolo_hub_state::HubState;

pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}

pub struct Fixture {
    pub state: Arc<HubState>,
    pub backend: Arc<Backend>,
    pub registry: AccessRegistry,
    pub socket: PathBuf,
    pub key: Arc<russh::keys::PrivateKey>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<anyhow::Result<()>>>,
    _directory: tempfile::TempDir,
    _home: tempfile::TempDir,
}

impl Fixture {
    pub async fn start() -> Self {
        let prepared = Prepared::new();
        let backend = prepared.backend.clone();
        prepared.start(backend).await
    }
    pub fn pin(&self) -> HubPin {
        HubPin {
            hub_id: self.state.hub_id(),
            public_key: self.state.public_key().try_into().unwrap(),
        }
    }
    pub async fn connection(&self) -> ClientConnection {
        let stream = tokio::net::UnixStream::connect(&self.socket).await.unwrap();
        yolo_hub_client_transport::connect(
            ClientIo::from_unix(stream).unwrap(),
            self.key.clone(),
            self.pin(),
            deadline(),
        )
        .await
        .unwrap()
    }
    pub async fn finish(mut self) {
        self.stop.take().unwrap().send(()).unwrap();
        tokio::time::timeout_at(deadline(), self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(!self.socket.exists());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
