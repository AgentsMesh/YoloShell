use super::constants::*;
use std::path::PathBuf;
use yolo_hub_configuration_store::{CONFIGURATION_DIRECTORY, ConfigurationStore};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_listener_binding::ListenerBindings;
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationState};
use yolo_hub_state::HubState;

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub hub: HubState,
    pub configuration: ConfigurationStore,
}
impl Fixture {
    pub fn new(listen: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let hub =
            HubState::open(&directory.path().join(HUB), &directory.path().join(HOST)).unwrap();
        let root = hub.directory().unwrap();
        let configuration = ConfigurationStore::open(
            root.ensure_descendant(&root.path().join(CONFIGURATION_DIRECTORY))
                .unwrap(),
            hub.peer_identity(),
            Some(Configuration {
                listen: listen.then(|| LOOPBACK.parse().unwrap()),
                ..Configuration::default()
            }),
        )
        .unwrap();
        Self {
            directory,
            hub,
            configuration,
        }
    }

    pub fn open(&self, fresh: bool) -> anyhow::Result<ListenerBindings> {
        ListenerBindings::open(
            self.hub.directory()?,
            self.hub.peer_identity(),
            &self.configuration.current()?,
            fresh,
        )
    }

    pub fn path(&self) -> PathBuf {
        self.hub
            .directory()
            .unwrap()
            .path()
            .join(yolo_hub_listener_binding::LISTENER_BINDING_FILE)
    }

    pub fn change(&self) -> ConfigurationChange {
        change(&self.configuration.current().unwrap())
    }
}

pub fn change(current: &ConfigurationState) -> ConfigurationChange {
    ConfigurationChange {
        request_id: uuid::Uuid::new_v4(),
        expected_revision: current.revision,
        configuration: current.configuration.clone(),
    }
}
