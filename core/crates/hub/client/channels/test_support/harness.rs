use super::{constants::*, peer::*};
use tokio::{
    net::UnixListener,
    sync::{mpsc, watch},
    task::JoinHandle,
    time::timeout_at,
};
use uuid::Uuid;
use yolo_hub_client_channels::*;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::Framed;

pub struct Harness {
    pub endpoint: Endpoint,
    pub commands: Option<mpsc::Sender<ControlRequest>>,
    pub maintenance: Option<mpsc::Sender<ControlRequest>>,
    pub refresh: Option<watch::Sender<Option<Uuid>>>,
    pub controls: mpsc::Receiver<ControlEvent>,
    pub catalogs: mpsc::Receiver<CatalogEvent>,
    pub listener: UnixListener,
    task: Option<JoinHandle<()>>,
    _root: tempfile::TempDir,
}
impl Harness {
    pub fn start() -> Self {
        let root = tempfile::tempdir_in(ROOT).unwrap();
        let endpoint = Endpoint::local(root.path().join(SOCKET), HubId::new());
        let listener = UnixListener::bind(endpoint.entry.local_socket().unwrap()).unwrap();
        let (commands, requests) = mpsc::channel(CAPACITY);
        let (maintenance, renewals) = mpsc::channel(CAPACITY);
        let (refresh, refreshes) = watch::channel(None);
        let (control_updates, controls) = mpsc::channel(CAPACITY);
        let (catalog_updates, catalogs) = mpsc::channel(CAPACITY);
        let task = tokio::spawn(run(
            endpoint.clone(),
            requests,
            renewals,
            refreshes,
            control_updates,
            catalog_updates,
        ));
        Self {
            endpoint,
            commands: Some(commands),
            maintenance: Some(maintenance),
            refresh: Some(refresh),
            controls,
            catalogs,
            listener,
            task: Some(task),
            _root: root,
        }
    }
    pub async fn accept(&self) -> Peer {
        let (stream, _) = timeout_at(deadline(), self.listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut peer = Framed::new(stream);
        hello(&mut peer, self.endpoint.hub).await;
        peer
    }
    pub async fn event(&mut self) -> ControlEvent {
        timeout_at(deadline(), self.controls.recv())
            .await
            .unwrap()
            .unwrap()
    }
    pub async fn ready(&mut self) -> Uuid {
        let ControlEvent::Ready { generation, .. } = self.event().await else {
            panic!()
        };
        generation
    }
    pub async fn send(&self, generation: Option<Uuid>, command: yolo_hub_rpc_protocol::Command) {
        self.commands
            .as_ref()
            .unwrap()
            .send(ControlRequest {
                validity: None,
                generation,
                destination: self.endpoint.hub,
                command,
            })
            .await
            .unwrap();
    }
    pub fn refresh(&self) -> Uuid {
        let request_id = Uuid::new_v4();
        self.refresh
            .as_ref()
            .unwrap()
            .send_replace(Some(request_id));
        request_id
    }
    pub async fn finish(mut self) {
        self.commands.take();
        self.maintenance.take();
        self.refresh.take();
        tokio::time::timeout(SHUTDOWN_TIMEOUT, self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    }
    pub async fn cancel(&mut self) {
        let task = self.task.take().unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
