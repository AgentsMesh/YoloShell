use super::constants::*;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_hub_agent_protocol::{Activity, AgentWorkingDirectory, Lifecycle, ObservedProvider};
use yolo_hub_connector_protocol::{
    Activation, ConnectorScope, Enrollment, Publication, ReportedState,
};
use yolo_hub_connector_registry::{Registry, RuntimeAdmission};
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_identity::{
    ConnectorId, ConnectorInstanceId, HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress,
    TerminalId,
};
use yolo_paths::{
    directory::PrivateDirectory, identity::FileIdentity, namespace_lease::NamespaceLease,
};

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub path: PathBuf,
    pub identity: PeerIdentity,
    pub scope: ConnectorScope,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().canonicalize().unwrap().join(REGISTRY_DIRECTORY);
        let identity =
            PeerIdentity::new(HubId::new(), [KEY_BYTE; ED25519_PUBLIC_KEY_BYTES]).unwrap();
        let scope = ConnectorScope {
            terminal: RuntimeTarget {
                address: TerminalAddress {
                    hub_id: identity.hub(),
                    terminal_id: TerminalId::new(),
                },
                runtime_epoch: RuntimeEpoch::new(),
            },
            provider: ObservedProvider::new(PROVIDER.into(), Some(PROVIDER_VERSION.into()), None)
                .unwrap(),
        };
        Self {
            root,
            path,
            identity,
            scope,
        }
    }
    pub fn open(&self) -> Registry {
        Registry::open(PrivateDirectory::ensure(&self.path).unwrap(), self.identity).unwrap()
    }
    pub fn admission(&self) -> RuntimeAdmission {
        RuntimeAdmission::Available(self.scope.terminal)
    }
    pub fn register(&self, registry: &mut Registry) -> Enrollment {
        registry
            .register(Uuid::new_v4(), self.scope.clone(), self.admission())
            .unwrap()
    }
    pub fn activate(
        &self,
        registry: &mut Registry,
        enrollment: &Enrollment,
        instance: ConnectorInstanceId,
    ) -> Activation {
        registry
            .activate(
                enrollment.connector_id,
                enrollment.credential,
                instance,
                self.admission(),
            )
            .unwrap()
    }
    pub fn member(&self, connector: ConnectorId) -> PathBuf {
        self.path
            .join(MEMBER_DIRECTORY)
            .join(connector.to_string())
            .with_extension(MEMBER_EXTENSION)
    }
    pub fn namespace(&self) -> NamespaceLease {
        NamespaceLease::capture(&[(TEST_NAMESPACE, &self.path)]).unwrap()
    }
    pub fn read(&self, path: &Path) -> serde_json::Value {
        self.namespace().read_json(path).unwrap()
    }
    pub fn write(&self, path: &Path, value: &impl serde::Serialize) {
        self.namespace().write_json(path, value).unwrap();
    }
    pub fn version(&self, path: &Path) -> FileIdentity {
        self.namespace()
            .read_json_with_identity::<serde_json::Value>(path)
            .unwrap()
            .1
    }
}

pub fn publication(activation: Activation, sequence: u64) -> Publication {
    Publication {
        connector_id: activation.connector_id,
        generation: activation.generation,
        agent: activation.agent,
        terminal: activation.terminal,
        sequence,
        state: ReportedState {
            lifecycle: Lifecycle::Running {},
            activity: Activity::Working,
            working_directory: Some(AgentWorkingDirectory::new(WORKING_DIRECTORY.into()).unwrap()),
        },
    }
}
pub fn exited(mut publication: Publication) -> Publication {
    publication.state.lifecycle = Lifecycle::Exited { exit_code: Some(0) };
    publication.state.activity = Activity::Idle;
    publication
}
