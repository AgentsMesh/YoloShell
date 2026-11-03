use super::constants::*;
use serde_json::Value;
use yolo_hub_state::{HUB_IDENTITY_FILE, HubState};
use yolo_paths::{directory::PrivateDirectory, namespace_lease::NamespaceLease};

#[test]
fn replacing_directory_at_original_path_does_not_adopt_identical_identity_bytes() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let path = root.join(STATE);
    let host = root.join(HOST);
    let state = HubState::open(&path, &host).unwrap();
    let bytes = std::fs::read(path.join(HUB_IDENTITY_FILE)).unwrap();
    drop(state);
    std::fs::rename(&path, root.join(RETIRED)).unwrap();
    let directory = PrivateDirectory::ensure(&path).unwrap();
    let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)]).unwrap();
    let record: Value = serde_json::from_slice(&bytes).unwrap();
    namespace
        .write_json(&path.join(HUB_IDENTITY_FILE), &record)
        .unwrap();
    let written = std::fs::read(path.join(HUB_IDENTITY_FILE)).unwrap();
    assert!(HubState::inspect_peer_identity(&path, &host).is_err());
    assert!(HubState::open(&path, &host).is_err());
    assert_eq!(
        std::fs::read(path.join(HUB_IDENTITY_FILE)).unwrap(),
        written
    );
}

#[test]
fn foreign_durable_proof_and_retired_schema_are_rejected_without_rebinding() {
    for retired_schema in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let path = root.join(STATE);
        let host = root.join(HOST);
        let state = HubState::open(&path, &host).unwrap();
        let namespace =
            NamespaceLease::from_directories(vec![(NAMESPACE, state.directory().unwrap())])
                .unwrap();
        let file = path.join(HUB_IDENTITY_FILE);
        let mut record: Value = namespace.read_json(&file).unwrap();
        drop(state);
        if retired_schema {
            record[SCHEMA] = RETIRED_SCHEMA.into();
        } else {
            let foreign = PrivateDirectory::ensure(&root.join(FOREIGN)).unwrap();
            record[BINDING][DIRECTORY] =
                serde_json::to_value(foreign.durable_identity().unwrap()).unwrap();
        }
        namespace.write_json(&file, &record).unwrap();
        let before = std::fs::read(&file).unwrap();
        assert!(HubState::inspect_peer_identity(&path, &host).is_err());
        assert!(HubState::open(&path, &host).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), before);
    }
}

#[test]
fn reopening_unchanged_state_keeps_exact_public_identity_and_private_binding_bytes() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let path = root.join(STATE);
    let host = root.join(HOST);
    let state = HubState::open(&path, &host).unwrap();
    let peer = state.peer_identity();
    let before = std::fs::read(path.join(HUB_IDENTITY_FILE)).unwrap();
    drop(state);
    assert_eq!(
        HubState::inspect_peer_identity(&path, &host).unwrap(),
        Some(peer)
    );
    let reopened = HubState::open(&path, &host).unwrap();
    assert_eq!(reopened.peer_identity(), peer);
    assert_eq!(std::fs::read(path.join(HUB_IDENTITY_FILE)).unwrap(), before);
}
