use super::constants::*;
use std::path::PathBuf;
use yolo_hub_configuration_store::{CONFIGURATION_DIRECTORY, ConfigurationStore};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_state::HubState;
use yolo_paths::{directory::PrivateDirectory, namespace_lease::NamespaceLease};

pub struct Fixture {
    pub path: PathBuf,
    pub hub: HubState,
    pub root: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().canonicalize().unwrap();
        let hub = HubState::open(&path.join(HUB_DIRECTORY), &path.join(HOST_DIRECTORY)).unwrap();
        let directory = hub.directory().unwrap();
        let path = directory.path().join(CONFIGURATION_DIRECTORY);
        directory.ensure_descendant(&path).unwrap();
        Self { path, hub, root }
    }

    pub fn directory(&self) -> PrivateDirectory {
        PrivateDirectory::open(&self.path).unwrap()
    }

    pub fn open(&self, seed: Option<Configuration>) -> ConfigurationStore {
        ConfigurationStore::open(self.directory(), self.hub.peer_identity(), seed).unwrap()
    }

    pub fn read(&self) -> serde_json::Value {
        self.namespace()
            .read_json(&self.path.join(RECORD_FILE))
            .unwrap()
    }

    pub fn write(&self, record: &serde_json::Value) {
        self.namespace()
            .write_json(&self.path.join(RECORD_FILE), record)
            .unwrap();
    }

    pub fn namespace(&self) -> NamespaceLease {
        NamespaceLease::from_directories(vec![(NAMESPACE, self.directory())]).unwrap()
    }
}
