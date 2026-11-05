use super::super::{
    federation_support::{RunningNode, deadline},
    files_runtime,
};
use crate::Harness;
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::watch, task::JoinHandle};
use yolo_hub_connector_service::ConnectorServer;
use yolo_hub_connector_store::CONNECTOR_DIRECTORY;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_state::HubState;

pub struct ConnectorNode {
    pub hub: Option<RunningNode>,
    pub identity: PeerIdentity,
    pub socket: PathBuf,
    stop: watch::Sender<bool>,
    task: Option<JoinHandle<anyhow::Result<()>>>,
}
impl ConnectorNode {
    pub async fn start(
        harness: &Harness,
        state: Arc<HubState>,
        config: Configuration,
    ) -> Self {
        let directory = state.directory().unwrap();
        let connectors = directory
            .ensure_descendant(&directory.path().join(CONNECTOR_DIRECTORY))
            .unwrap();
        let identity = state.peer_identity();
        let backend = files_runtime::prepared(harness, &state, &config)
            .await
            .with_connectors(connectors, state.peer_identity())
            .unwrap();
        let server = ConnectorServer::bind(state.clone(), backend.clone())
            .await
            .unwrap();
        let socket = server.socket_path();
        let hub = RunningNode::start_prepared(state, config, backend).await;
        let (stop, mut stopped) = watch::channel(false);
        let task = tokio::spawn(server.serve_until(async move {
            let _ = stopped.changed().await;
        }));
        Self {
            hub: Some(hub),
            identity,
            socket,
            stop,
            task: Some(task),
        }
    }
    pub fn running(&self) -> &RunningNode {
        self.hub.as_ref().unwrap()
    }
    pub async fn stop(mut self) {
        self.stop.send_replace(true);
        tokio::time::timeout_at(deadline(), self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        self.hub.take().unwrap().stop().await;
    }
}
impl Drop for ConnectorNode {
    fn drop(&mut self) {
        self.stop.send_replace(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
