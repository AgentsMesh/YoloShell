use super::*;
#[path = "inspection_constants.rs"]
mod constants;
use constants::PUBLIC_PEER_FIELDS;

#[test]
fn inspection_is_read_only_and_works_while_the_service_holds_its_singleton() {
    let (_root, path, host) = fixture();
    assert_eq!(HubState::inspect_identity(&path, &host).unwrap(), None);
    assert_eq!(HubState::inspect_peer_identity(&path, &host).unwrap(), None);
    assert!(!path.exists());
    let state = HubState::open(&path, &host).unwrap();
    let before = std::fs::read(path.join(HUB_IDENTITY_FILE)).unwrap();
    let public = HubState::inspect_peer_identity(&path, &host)
        .unwrap()
        .unwrap();
    assert_eq!(public, state.peer_identity());
    assert_eq!(public.key().as_slice(), state.public_key());
    let wire = serde_json::to_value(public).unwrap();
    let keys = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(keys, PUBLIC_PEER_FIELDS.into_iter().collect());
    assert_eq!(std::fs::read(path.join(HUB_IDENTITY_FILE)).unwrap(), before);
    assert_eq!(
        HubState::inspect_identity(&path, &host).unwrap(),
        Some(state.hub_id())
    );
    assert!(HubState::open(&path, &host).is_err());
}

#[test]
fn inspection_rejects_wrong_home_and_corrupt_key_without_regenerating_identity() {
    let (root, path, host) = fixture();
    let state = HubState::open(&path, &host).unwrap();
    assert!(HubState::inspect_identity(&path, &root.path().join(DIFFERENT_HOST)).is_err());
    assert!(HubState::inspect_peer_identity(&path, &root.path().join(DIFFERENT_HOST)).is_err());
    let file = path.join(HUB_IDENTITY_FILE);
    let mut record: Record = state.namespace.read_json(&file).unwrap();
    record.key = INVALID_KEY_BYTES.to_vec();
    state.namespace.write_json(&file, &record).unwrap();
    assert!(HubState::inspect_identity(&path, &host).is_err());
    assert!(HubState::inspect_peer_identity(&path, &host).is_err());
    assert_eq!(
        state.namespace.read_json::<Record>(&file).unwrap().hub_id,
        state.hub_id()
    );
}

#[test]
fn inspecting_an_empty_private_directory_does_not_create_identity_or_locks() {
    let (_root, path, host) = fixture();
    PrivateDirectory::ensure(&path).unwrap();
    assert_eq!(HubState::inspect_identity(&path, &host).unwrap(), None);
    assert_eq!(HubState::inspect_peer_identity(&path, &host).unwrap(), None);
    assert_eq!(std::fs::read_dir(&path).unwrap().count(), 0);
    assert!(!host.exists());
}

#[test]
fn inspecting_a_cloned_identity_rejects_it_without_rewriting_the_record() {
    let (root, path, host) = fixture();
    let state = HubState::open(&path, &host).unwrap();
    let record: Record = state
        .namespace
        .read_json(&path.join(HUB_IDENTITY_FILE))
        .unwrap();
    let clone_path = root.path().canonicalize().unwrap().join(CLONE_DIRECTORY);
    let clone = PrivateDirectory::ensure(&clone_path).unwrap();
    let namespace = NamespaceLease::from_directories(vec![(STATE_NAMESPACE, clone)]).unwrap();
    let identity = clone_path.join(HUB_IDENTITY_FILE);
    namespace.write_json(&identity, &record).unwrap();
    let before = std::fs::read(&identity).unwrap();
    assert!(HubState::inspect_identity(&clone_path, &host).is_err());
    assert!(HubState::inspect_peer_identity(&clone_path, &host).is_err());
    assert_eq!(std::fs::read(&identity).unwrap(), before);
    assert_eq!(
        HubState::inspect_identity(&path, &host).unwrap(),
        Some(state.hub_id())
    );
}

#[test]
fn inspection_rejects_invalid_schema_and_symlinked_identity() {
    let (_root, path, host) = fixture();
    let state = HubState::open(&path, &host).unwrap();
    let identity = path.join(HUB_IDENTITY_FILE);
    let mut record: Record = state.namespace.read_json(&identity).unwrap();
    record.schema = STATE_SCHEMA_VERSION + 1;
    state.namespace.write_json(&identity, &record).unwrap();
    let before = std::fs::read(&identity).unwrap();
    assert!(HubState::inspect_identity(&path, &host).is_err());
    assert!(HubState::inspect_peer_identity(&path, &host).is_err());
    assert_eq!(std::fs::read(&identity).unwrap(), before);
    let moved = path.join(CLONE_DIRECTORY);
    std::fs::rename(&identity, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &identity).unwrap();
    assert!(HubState::inspect_identity(&path, &host).is_err());
    assert!(HubState::inspect_peer_identity(&path, &host).is_err());
    assert!(HubState::inspect_identity(Path::new(STATE_DIRECTORY), &host).is_err());
    assert!(HubState::inspect_peer_identity(Path::new(STATE_DIRECTORY), &host).is_err());
}

#[test]
fn public_inspection_observes_a_changed_key_even_when_the_hub_id_is_unchanged() {
    let (_root, path, host) = fixture();
    let state = HubState::open(&path, &host).unwrap();
    let file = path.join(HUB_IDENTITY_FILE);
    let mut record: Record = state.namespace.read_json(&file).unwrap();
    let key = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    record.key = key.as_ref().to_vec();
    state.namespace.write_json(&file, &record).unwrap();
    let before = std::fs::read(&file).unwrap();
    let inspected = HubState::inspect_peer_identity(&path, &host)
        .unwrap()
        .unwrap();
    assert_eq!(inspected.hub(), state.hub_id());
    assert_ne!(inspected.key().as_slice(), state.public_key());
    let parsed = Ed25519KeyPair::from_pkcs8(&record.key).unwrap();
    assert_eq!(inspected.key().as_slice(), parsed.public_key().as_ref());
    assert_eq!(std::fs::read(&file).unwrap(), before);
}
