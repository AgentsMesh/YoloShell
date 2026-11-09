use super::listener_constants::*;
use crate::*;
use yolo_hub_configuration_store::ConfigurationStore;
use yolo_hub_management_protocol::ConfigurationState;

pub struct InitialStartup {
    pub directory: tempfile::TempDir,
    pub state: Arc<HubState>,
    pub backend: LocalBackend,
    pub store: ConfigurationStore,
}

impl InitialStartup {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().join(HOST_DIRECTORY);
        let state = Arc::new(HubState::open(&directory.path().join(HUB_DIRECTORY), &home).unwrap());
        let backend =
            LocalBackend::new(state.hub_id(), &home, &directory.path().join(HOST_SOCKET)).unwrap();
        let root = state.directory().unwrap();
        let store = ConfigurationStore::open(
            root.ensure_descendant(&root.path().join(CONFIGURATION_DIRECTORY))
                .unwrap(),
            state.peer_identity(),
            Some(Configuration {
                listen: Some(LOOPBACK.parse().unwrap()),
                ..Configuration::default()
            }),
        )
        .unwrap();
        Self {
            directory,
            state,
            backend,
            store,
        }
    }

    pub fn release(self) -> (tempfile::TempDir, LocalBackend, ConfigurationState) {
        let configuration = self.store.current().unwrap();
        let Self {
            directory,
            state,
            backend,
            store,
        } = self;
        drop((store, state));
        (directory, backend, configuration)
    }
}
