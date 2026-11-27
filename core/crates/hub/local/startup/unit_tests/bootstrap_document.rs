use crate::{BootstrapDocument, ConnectionDocument};
use yolo_hub_state::HubState;
use yolo_paths::directory::PrivateDirectory;

#[path = "../test_support/constants.rs"]
mod fixture_constants;
use fixture_constants::BOOTSTRAP_STATE_DIRECTORY;

const HOME: &str = "host";
const OTHER: &str = "other";

#[test]
fn startup_receipt_binds_requested_directories_to_the_actual_short_socket() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap();
    let home = path.join(HOME);
    PrivateDirectory::ensure(&home).unwrap();
    let state_path = path.join(BOOTSTRAP_STATE_DIRECTORY);
    let state = HubState::open(&state_path, &home).unwrap();
    let connection = ConnectionDocument::local(state.hub_id(), state.socket_path());
    let receipt = BootstrapDocument::new(&state_path, &home, connection.clone()).unwrap();
    assert_eq!(receipt.connection, connection);
    assert_eq!(receipt.state_path, state_path);
    assert_eq!(receipt.host_home, home);
    assert_eq!(
        serde_json::from_slice::<BootstrapDocument>(&serde_json::to_vec(&receipt).unwrap())
            .unwrap(),
        receipt
    );
    assert!(BootstrapDocument::new(&state_path, &path.join(OTHER), connection.clone()).is_err());
    let mutations: &[fn(&mut ConnectionDocument)] = &[
        |c| c.entry_hub = yolo_hub_identity::HubId::new(),
        |c| c.local_origin = yolo_hub_identity::HubId::new(),
        |c| c.schema_version += 1,
        |c| c.socket_path = c.socket_path.with_file_name(OTHER),
    ];
    for mutate in mutations {
        let mut changed = connection.clone();
        mutate(&mut changed);
        assert!(BootstrapDocument::new(&state_path, &home, changed).is_err());
    }
}
