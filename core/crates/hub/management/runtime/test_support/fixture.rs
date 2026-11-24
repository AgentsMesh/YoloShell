use crate::*;
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_hub_rpc_protocol::{Command, Reply, Request, Response};

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub state: Arc<HubState>,
    pub hub: Arc<ManagedHub>,
    pub service: ManagementService,
}
impl Fixture {
    pub async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().join("host");
        let state = Arc::new(HubState::open(&directory.path().join("hub"), &home).unwrap());
        let backend = LocalBackend::new(
            state.hub_id(),
            &home,
            &directory.path().join("absent-host.sock"),
        )
        .unwrap();
        let (hub, service) = ManagedHub::open(state.clone(), backend, None)
            .await
            .unwrap();
        Self {
            directory,
            state,
            hub,
            service,
        }
    }
}
pub async fn call(hub: &ManagedHub, command: Command) -> ManagementResult {
    let request = Request::new(hub.state.hub_id(), command).unwrap();
    let Operation::Reply(reply) = hub.dispatch(request.clone()).await? else {
        panic!()
    };
    Response::new(&request, hub.state.hub_id(), reply.clone())
        .validate_for(&request)
        .unwrap();
    let Reply::Configuration { snapshot } = reply else {
        panic!()
    };
    Ok(snapshot)
}
pub fn change(snapshot: &ConfigurationSnapshot) -> ConfigurationChange {
    ConfigurationChange {
        request_id: uuid::Uuid::new_v4(),
        expected_revision: snapshot.state.revision,
        configuration: snapshot.state.configuration.clone(),
    }
}
