use super::{
    recovery_constants::*,
    recovery_peer::{Peer, State},
};
use crate::{RemoteDirectories, configuration};
use std::sync::{Arc, Mutex};
use yolo_hub_enrollment_store::{Admission, PreparedEnrollment, SubmitPermit};
use yolo_hub_local_startup::ConnectionDocument;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationSnapshot, ConfigurationState, FIRST_CONFIGURATION_REVISION,
};
use yolo_remote_provisioning_protocol::RemoteBootstrapRequest;
use yolo_hub_rpc_protocol::Method;
use yolo_hub_state::HubState;

pub struct Fixture {
    pub peer: Peer,
    pub shared: Arc<Mutex<State>>,
    pub hub: Arc<HubState>,
    pub directories: RemoteDirectories,
    pub request: RemoteBootstrapRequest,
    pub root: tempfile::TempDir,
}
impl Fixture {
    pub async fn new(lose_reply: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().canonicalize().unwrap();
        let directories = RemoteDirectories {
            state: base.join(STATE_DIRECTORY),
            host_home: base.join(HOST_DIRECTORY),
            install_root: base.join(ABSENT_INSTALL),
        };
        let hub = Arc::new(HubState::open(&directories.state, &directories.host_home).unwrap());
        let mut fields = yolo_hub_remote_startup_test_support::request()
            .fields()
            .clone();
        fields.expected_peer = Some(hub.peer_identity());
        let request = RemoteBootstrapRequest::new(fields).unwrap();
        let shared = Arc::new(Mutex::new(State {
            snapshot: ConfigurationSnapshot {
                state: ConfigurationState {
                    schema_version: CONFIGURATION_SCHEMA,
                    identity: hub.peer_identity(),
                    revision: FIRST_CONFIGURATION_REVISION,
                    last_change: uuid::Uuid::new_v4(),
                    configuration: Default::default(),
                },
                applied: true,
                listen_address: None,
            },
            build: yolo_hub_remote_startup_test_support::receipt(&request).build,
            methods: Vec::new(),
            lose_reply,
            before_snapshot: None,
            disconnect_after: None,
            connections: 0,
        }));
        let peer = Peer::start(hub.clone(), shared.clone()).await;
        Self {
            peer,
            shared,
            hub,
            directories,
            request,
            root,
        }
    }
    pub fn connection(&self) -> ConnectionDocument {
        ConnectionDocument::local(self.hub.hub_id(), self.hub.socket_path())
    }
    pub fn prepared(&self) -> PreparedEnrollment {
        let snapshot = self.shared.lock().unwrap().snapshot.clone();
        let change = configuration::plan(&snapshot, &self.request).unwrap();
        PreparedEnrollment {
            request: self.request.clone(),
            peer: self.hub.peer_identity(),
            snapshot,
            change,
        }
    }
    pub fn prepare(&self) -> SubmitPermit {
        let Admission::Fresh(claim) = yolo_hub_enrollment_store::claim(
            self.hub.directory().unwrap(),
            self.hub.peer_identity(),
            &self.request,
        )
        .unwrap() else {
            panic!()
        };
        claim.prepare(self.prepared()).unwrap()
    }
    pub fn apply(&self, prepared: &PreparedEnrollment) {
        let mut state = self.shared.lock().unwrap();
        let change = prepared.change.as_ref().unwrap();
        state.snapshot.state.configuration = change.configuration.clone();
        state.snapshot.state.revision = prepared.revision();
        state.snapshot.state.last_change = change.request_id;
        state.snapshot.listen_address = Some((std::net::Ipv4Addr::LOCALHOST, PORT).into());
    }
    pub fn configured(&self) -> usize {
        self.shared
            .lock()
            .unwrap()
            .methods
            .iter()
            .filter(|method| **method == Method::Configure)
            .count()
    }
    pub fn file(&self) -> std::path::PathBuf {
        self.directories
            .state
            .join(yolo_hub_enrollment_store::ENROLLMENT_DIRECTORY)
            .join(self.request.fields().request_id.to_string())
            .join(RECORD_FILE)
    }
}
