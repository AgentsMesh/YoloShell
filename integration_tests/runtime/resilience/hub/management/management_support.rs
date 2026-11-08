use super::federation_support::deadline;
use crate::Harness;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{sync::watch, task::JoinHandle};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_hub_management_runtime::ManagedHub;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_server::HubServer;
use yolo_hub_state::HubState;

pub struct ManagedNode {
    pub hub: HubId,
    pub address: SocketAddr,
    pub socket: PathBuf,
    stop: watch::Sender<bool>,
    service: Option<JoinHandle<anyhow::Result<()>>>,
    local: Option<JoinHandle<anyhow::Result<()>>>,
}
impl ManagedNode {
    pub async fn start(
        harness: &Harness,
        state: Arc<HubState>,
        configuration: Configuration,
    ) -> Self {
        Self::start_with_helper(
            harness,
            state,
            configuration,
            super::files_runtime::helper(),
        )
        .await
    }
    pub async fn start_with_helper(
        harness: &Harness,
        state: Arc<HubState>,
        configuration: Configuration,
        file_helper: PathBuf,
    ) -> Self {
        let hub = state.hub_id();
        let socket = state.socket_path();
        let backend = super::files_runtime::backend(harness, &state)
            .with_file_helper(file_helper)
            .unwrap();
        let (managed, service) = ManagedHub::open(state.clone(), backend, Some(configuration))
            .await
            .unwrap();
        let address = service.listen_address().unwrap().unwrap();
        let server = HubServer::bind_dispatcher(state, managed).await.unwrap();
        let (stop, stopped) = watch::channel(false);
        let mut local_stop = stopped.clone();
        let local = tokio::spawn(server.serve_until(async move {
            let _ = local_stop.changed().await;
        }));
        let service = tokio::spawn(service.run(stopped));
        Self {
            hub,
            address,
            socket,
            stop,
            local: Some(local),
            service: Some(service),
        }
    }
    pub async fn client(&self) -> yolo_hub_client::LocalClient {
        yolo_hub_client::Client::connect_local(&self.socket, self.hub, deadline())
            .await
            .unwrap()
    }
    pub async fn snapshot(&self) -> ConfigurationSnapshot {
        let Reply::Configuration { snapshot } = self
            .client()
            .await
            .call(self.hub, Command::GetConfiguration {}, deadline())
            .await
            .unwrap()
        else {
            panic!()
        };
        snapshot
    }
    pub async fn configure(&self, configuration: Configuration) -> ConfigurationSnapshot {
        let current = self.snapshot().await;
        let change = ConfigurationChange {
            request_id: uuid::Uuid::new_v4(),
            expected_revision: current.state.revision,
            configuration,
        };
        let Reply::Configuration { snapshot } = self
            .client()
            .await
            .call(self.hub, Command::Configure { change }, deadline())
            .await
            .unwrap()
        else {
            panic!()
        };
        assert!(snapshot.applied);
        snapshot
    }
    pub async fn stop(mut self) {
        self.stop.send_replace(true);
        tokio::time::timeout_at(deadline(), self.local.take().unwrap())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        tokio::time::timeout_at(deadline(), self.service.take().unwrap())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
impl Drop for ManagedNode {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}
