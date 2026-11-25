use super::{State, constants::*};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_hub_agent_protocol::ObservedProvider;
use yolo_hub_connector_protocol::{ConnectorCredential, ConnectorScope};
use yolo_hub_connector_store::{CONNECTOR_DIRECTORY, Seed, Store};
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_identity::{
    AgentAddress, AgentId, ConnectorId, HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress,
    TerminalId,
};
use yolo_paths::{
    directory::PrivateDirectory, identity::FileIdentity, namespace_lease::NamespaceLease,
};

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub path: PathBuf,
    pub identity: PeerIdentity,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root
            .path()
            .canonicalize()
            .unwrap()
            .join(CONNECTOR_DIRECTORY);
        let identity =
            PeerIdentity::new(HubId::new(), [KEY_BYTE; ED25519_PUBLIC_KEY_BYTES]).unwrap();
        Self {
            root,
            path,
            identity,
        }
    }
    pub fn directory(&self) -> PrivateDirectory {
        PrivateDirectory::ensure(&self.path).unwrap()
    }
    pub fn open(&self) -> anyhow::Result<Store<State>> {
        Store::open(self.directory(), self.identity)
    }
    pub fn namespace(&self) -> NamespaceLease {
        NamespaceLease::capture(&[(TEST_NAMESPACE, &self.path)]).unwrap()
    }
    pub fn index(&self) -> PathBuf {
        self.path.join(INDEX_FILE)
    }
    pub fn member(&self, seed: &Seed) -> PathBuf {
        self.path
            .join(MEMBERS_DIRECTORY)
            .join(seed.connector_id.to_string())
            .with_extension(MEMBER_EXTENSION)
    }
    pub fn read(&self, path: &Path) -> Value {
        self.namespace().read_json(path).unwrap()
    }
    pub fn file_identity(&self, path: &Path) -> FileIdentity {
        self.namespace()
            .read_json_with_identity::<Value>(path)
            .unwrap()
            .1
    }
    pub fn write(&self, path: &Path, value: &impl serde::Serialize) {
        self.namespace().write_json(path, value).unwrap();
    }
    pub fn replace_member(&self, seed: &Seed, state: &State) {
        self.write(
            &self.member(seed),
            &json!({ (SEED_FIELD): seed, (STATE_FIELD): state }),
        );
    }
    pub fn sibling(&self, name: &str) -> PathBuf {
        self.root.path().canonicalize().unwrap().join(name)
    }
    pub fn seed(&self) -> Seed {
        Seed {
            connector_id: ConnectorId::new(),
            request_id: Uuid::new_v4(),
            agent: AgentAddress {
                hub_id: self.identity.hub(),
                agent_id: AgentId::new(),
            },
            scope: ConnectorScope {
                terminal: RuntimeTarget {
                    address: TerminalAddress {
                        hub_id: self.identity.hub(),
                        terminal_id: TerminalId::new(),
                    },
                    runtime_epoch: RuntimeEpoch::new(),
                },
                provider: ObservedProvider::new(PROVIDER.into(), None, None).unwrap(),
            },
            credential: ConnectorCredential::generate(),
        }
    }
}
