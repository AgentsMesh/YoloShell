use crate::{ConfigurationStore, constants::RECORD_FILE, record::Record};
use std::{
    path::PathBuf,
};
use uuid::Uuid;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_federation_settings::{Configuration, Peer};
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationState};
use yolo_paths::{directory::PrivateDirectory, namespace_lease::NamespaceLease};

pub(crate) use super::constants;
use constants::*;

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub path: PathBuf,
    pub identity: PeerIdentity,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().canonicalize().unwrap().join(CONFIG_DIR);
        let identity = PeerIdentity::new(
            HubId::from_uuid(Uuid::from_u128(HUB_VALUE)).unwrap(),
            [KEY_BYTE; ED25519_PUBLIC_KEY_BYTES],
        )
        .unwrap();
        Self {
            root,
            path,
            identity,
        }
    }
    pub fn directory(&self) -> PrivateDirectory {
        PrivateDirectory::ensure(&self.path).unwrap()
    }
    pub fn open(&self, seed: Option<Configuration>) -> anyhow::Result<ConfigurationStore> {
        ConfigurationStore::open(self.directory(), self.identity, seed)
    }
    pub fn namespace(&self) -> NamespaceLease {
        NamespaceLease::capture(&[(TEST_NAMESPACE, &self.path)]).unwrap()
    }
    pub fn record(&self) -> Record {
        self.namespace()
            .read_json(&self.path.join(RECORD_FILE))
            .unwrap()
    }
    pub fn replace(&self, record: &impl serde::Serialize) {
        self.namespace()
            .write_json(&self.path.join(RECORD_FILE), record)
            .unwrap();
    }
    pub fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.path.join(RECORD_FILE)).unwrap()
    }
    pub fn sibling(&self, name: &str) -> PathBuf {
        self.root.path().canonicalize().unwrap().join(name)
    }
}
pub fn changed() -> Configuration {
    Configuration {
        trusted_origins: vec![other_identity()],
        ..Configuration::default()
    }
}
pub fn change(state: &ConfigurationState) -> ConfigurationChange {
    ConfigurationChange {
        request_id: Uuid::new_v4(),
        expected_revision: state.revision,
        configuration: changed(),
    }
}
pub fn other_identity() -> PeerIdentity {
    PeerIdentity::new(
        HubId::from_uuid(Uuid::from_u128(OTHER_HUB_VALUE)).unwrap(),
        [OTHER_KEY_BYTE; ED25519_PUBLIC_KEY_BYTES],
    )
    .unwrap()
}
pub fn invalid_configuration(local: PeerIdentity) -> Configuration {
    Configuration {
        peers: vec![Peer {
            identity: local,
            dial: None,
            consume: true,
            exports: vec![],
        }],
        ..Configuration::default()
    }
}
