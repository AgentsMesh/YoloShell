use super::RunningNode;
use crate::hub::catalog_gate::{CatalogGate, GatedDispatcher};
use std::sync::Arc;
use tokio::{net::TcpListener, sync::watch};
use yolo_hub_federation::Federation;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_server::HubServer;
use yolo_hub_state::HubState;

impl RunningNode {
    pub async fn start_prepared(
        state: Arc<HubState>,
        config: Configuration,
        listener: TcpListener,
        backend: LocalBackend,
    ) -> Self {
        let socket = state.socket_path();
        let node = Federation::new(state.clone(), backend, config)
            .await
            .unwrap();
        let catalog_gate = CatalogGate::new();
        let server = HubServer::bind_dispatcher(
            state,
            GatedDispatcher::new(node.clone(), catalog_gate.clone()),
        )
        .await
        .unwrap();
        let (stop, stopped) = watch::channel(false);
        let network = tokio::spawn(node.clone().run(Some(listener), stopped.clone()));
        let mut local_stop = stopped;
        let local = tokio::spawn(server.serve_until(async move {
            let _ = local_stop.changed().await;
        }));
        Self {
            node,
            catalog_gate,
            socket,
            stop,
            local: Some(local),
            network: Some(network),
        }
    }
}
