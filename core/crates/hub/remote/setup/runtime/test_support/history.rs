use crate::{constants::*, fixture::Fixture};
use std::sync::Arc;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_remote_setup_protocol::RemoteSetupHistory;
use yolo_hub_remote_setup_runtime::RemoteSetupRuntime;
use yolo_hub_remote_setup_store::{SetupOperation, SetupRecord};
use yolo_hub_rpc_protocol::{Command, Reply};

impl Fixture {
    pub async fn history(&self) -> RemoteSetupHistory {
        self.client()
            .await
            .inspect_remote_setup(self.request.request.fields().request_id, deadline())
            .await
            .unwrap()
    }

    pub async fn seed_prepared(&self) {
        self.seed_prepared_for(self.state.peer_identity()).await;
    }

    pub async fn seed_prepared_for(&self, owner: yolo_hub_delegation::PeerIdentity) {
        let (registration, reservation) = self
            .profiles
            .reserve_profile(&self.request.profile().unwrap())
            .unwrap();
        drop(reservation);
        let parent = self.state.directory().unwrap();
        let root = parent
            .open_descendant(
                &parent
                    .path()
                    .join(yolo_hub_remote_setup_store::REMOTE_SETUP_DIRECTORY),
            )
            .unwrap();
        let mut request = self.request.clone();
        let mut provision = request.request.fields().clone();
        let mut bootstrap = provision.bootstrap.fields().clone();
        bootstrap.consumer = owner;
        provision.bootstrap =
            yolo_remote_provisioning_protocol::RemoteBootstrapRequest::new(bootstrap).unwrap();
        request.request =
            yolo_remote_provisioning_protocol::ProvisionRequest::new(provision).unwrap();
        let mut baseline = self.configuration().await;
        baseline.state.identity = owner;
        let record = SetupRecord::new(owner, request, registration, baseline).unwrap();
        drop(SetupOperation::open(root, record).unwrap());
    }

    pub async fn fresh_reader_history(&self) -> RemoteSetupHistory {
        let reader = RemoteSetupRuntime::open(
            self.state.clone(),
            self.profiles.clone(),
            self.profile_dispatch.clone(),
        )
        .unwrap();
        let Reply::RemoteSetupHistory { history } = crate::fixture::call(
            &(reader as Arc<dyn Dispatcher>),
            Command::InspectRemoteSetup {
                operation_id: self.request.request.fields().request_id,
            },
        )
        .await
        .unwrap() else {
            panic!()
        };
        history
    }
}
