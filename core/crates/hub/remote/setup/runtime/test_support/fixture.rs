use super::{broker::Broker, constants::*};
use std::sync::Arc;
use tokio::{
    sync::{oneshot, watch},
    task::JoinHandle,
};
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_management_runtime::ManagedHub;
use yolo_hub_remote_setup_protocol::RemoteSetupRequest;
use yolo_hub_remote_setup_runtime::RemoteSetupRuntime;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_ssh_profile_protocol::{SshBrokerProfile, SshProfileChange, SshProfileConfiguration};
use yolo_hub_ssh_profile_runtime::SshProfileRuntime;
use yolo_hub_state::HubState;
#[path = "fixture_calls.rs"]
mod calls;
pub use calls::call;

pub struct Fixture {
    pub runtime: Arc<RemoteSetupRuntime>,
    pub profiles: Arc<SshProfileRuntime>,
    pub profile_dispatch: Arc<dyn Dispatcher>,
    pub managed: Arc<ManagedHub>,
    pub state: Arc<HubState>,
    pub request: RemoteSetupRequest,
    pub broker: Broker,
    server_stop: Option<oneshot::Sender<()>>,
    server: Option<JoinHandle<anyhow::Result<()>>>,
    service_stop: watch::Sender<bool>,
    service: Option<JoinHandle<anyhow::Result<()>>>,
    pub root: tempfile::TempDir,
}
impl Fixture {
    pub async fn new(published: bool, blocked: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().canonicalize().unwrap();
        let home = base.join(HOST_DIRECTORY);
        let state = Arc::new(HubState::open(&base.join(HUB_DIRECTORY), &home).unwrap());
        let backend = yolo_hub_local_backend::LocalBackend::new(
            state.hub_id(),
            &home,
            &base.join(UNUSED_HOST_SOCKET),
        )
        .unwrap();
        let (managed, service) = ManagedHub::open(state.clone(), backend, None)
            .await
            .unwrap();
        let (service_stop, stopped) = watch::channel(false);
        let service = tokio::spawn(service.run(stopped));
        let profiles = SshProfileRuntime::open(state.clone(), None).unwrap();
        let profile_dispatch = profiles.dispatcher(managed.clone()).unwrap();
        let broker = Broker::start(&base.join(BROKER_DIRECTORY), published, blocked).await;
        let mut request = yolo_hub_remote_setup_protocol_test_support::fixture::request();
        let mut fields = request.request.fields().clone();
        let mut bootstrap = fields.bootstrap.fields().clone();
        bootstrap.consumer = state.peer_identity();
        fields.bootstrap =
            yolo_remote_provisioning_protocol::RemoteBootstrapRequest::new(bootstrap).unwrap();
        request.request = yolo_remote_provisioning_protocol::ProvisionRequest::new(fields).unwrap();
        let registration = SshBrokerProfile {
            profile: request.profile().unwrap(),
            broker_state: broker.paths.state_dir().into(),
            config_revision: request.request.fields().profile.config_revision.clone(),
            launch: None,
        };
        let Reply::SshProfiles { state: current } =
            call(&profile_dispatch, Command::GetSshProfiles {})
                .await
                .unwrap()
        else {
            panic!()
        };
        call(
            &profile_dispatch,
            Command::ConfigureSshProfiles {
                change: SshProfileChange {
                    request_id: uuid::Uuid::new_v4(),
                    expected_revision: current.revision,
                    configuration: SshProfileConfiguration {
                        profiles: vec![registration],
                        ..Default::default()
                    },
                },
            },
        )
        .await
        .unwrap();
        let runtime =
            RemoteSetupRuntime::open(state.clone(), profiles.clone(), profile_dispatch.clone())
                .unwrap();
        let server = yolo_hub_server::HubServer::bind_dispatcher(state.clone(), runtime.clone())
            .await
            .unwrap();
        let (server_stop, stop) = oneshot::channel();
        let server = tokio::spawn(server.serve_until(async {
            let _ = stop.await;
        }));
        Self {
            runtime,
            profiles,
            profile_dispatch,
            managed,
            state,
            request,
            broker,
            server_stop: Some(server_stop),
            server: Some(server),
            service_stop,
            service: Some(service),
            root,
        }
    }
    pub fn record(&self) -> std::path::PathBuf {
        self.state
            .directory()
            .unwrap()
            .path()
            .join(yolo_hub_remote_setup_store::REMOTE_SETUP_DIRECTORY)
            .join(self.request.request.fields().request_id.to_string())
            .join(RECORD_FILE)
    }
    pub async fn finish(mut self) {
        let _ = self.server_stop.take().unwrap().send(());
        self.server.take().unwrap().await.unwrap().unwrap();
        self.service_stop.send_replace(true);
        self.service.take().unwrap().await.unwrap().unwrap();
        self.broker.finish().await;
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(stop) = self.server_stop.take() {
            let _ = stop.send(());
        }
        self.service_stop.send_replace(true);
        if let Some(task) = self.server.take() {
            task.abort();
        }
        if let Some(task) = self.service.take() {
            task.abort();
        }
    }
}
