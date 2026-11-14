use super::federation_support::deadline;
use std::sync::Arc;
use tokio::sync::{Notify, watch};
use yolo_hub_dispatch::{Dispatcher, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Request};

pub struct CatalogGate {
    paused: watch::Sender<bool>,
    started: Notify,
}
impl CatalogGate {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            paused: watch::channel(false).0,
            started: Notify::new(),
        })
    }
    pub fn pause(self: &Arc<Self>) -> Pause {
        self.paused.send_replace(true);
        Pause(self.clone())
    }
    async fn wait(&self) {
        let mut paused = self.paused.subscribe();
        if !*paused.borrow() {
            return;
        }
        self.started.notify_one();
        loop {
            if !*paused.borrow_and_update() {
                return;
            }
            if paused.changed().await.is_err() {
                return;
            }
        }
    }
}
pub struct Pause(Arc<CatalogGate>);
impl Pause {
    pub async fn started(&self) {
        tokio::time::timeout_at(deadline(), self.0.started.notified())
            .await
            .unwrap();
    }
}
impl Drop for Pause {
    fn drop(&mut self) {
        self.0.paused.send_replace(false);
    }
}

pub struct GatedDispatcher {
    inner: Arc<dyn Dispatcher>,
    gate: Arc<CatalogGate>,
}
impl GatedDispatcher {
    pub fn new(inner: Arc<dyn Dispatcher>, gate: Arc<CatalogGate>) -> Arc<Self> {
        Arc::new(Self { inner, gate })
    }
}
impl Dispatcher for GatedDispatcher {
    fn hub_id(&self) -> HubId {
        self.inner.hub_id()
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            if matches!(
                request.fields().command,
                Command::List { .. } | Command::ListAgents { .. } | Command::ListHubs { .. }
            ) {
                self.gate.wait().await;
            }
            self.inner.dispatch(request).await
        })
    }
}
