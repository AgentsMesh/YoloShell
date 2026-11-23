use super::*;
mod inspection;
mod socket_namespace;
use ring::signature::{ED25519, UnparsedPublicKey};

const MESSAGE: &[u8] = b"Hub enrollment challenge";
const CHANGED_MESSAGE: &[u8] = b"another enrollment challenge";
const STATE_DIRECTORY: &str = "hub-state";
const CLONE_DIRECTORY: &str = "clone";
const HOST_DIRECTORY: &str = "host";
const DIFFERENT_HOST: &str = "other-host";
const INVALID_KEY_BYTES: &[u8] = b"invalid key";

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let root_path = root.path().canonicalize().unwrap();
    let path = root_path.join(STATE_DIRECTORY);
    let host = root_path.join(HOST_DIRECTORY);
    (root, path, host)
}

#[test]
fn identity_and_signing_key_survive_restart() {
    let (_root, path, host) = fixture();
    let state = HubState::open(&path, &host).unwrap();
    let hub = state.hub_id();
    let key = state.public_key().to_vec();
    let signature = state.sign(MESSAGE).unwrap();
    let transport = state.transport_key().unwrap();
    assert_eq!(
        transport
            .public_key()
            .key_data()
            .ed25519()
            .unwrap()
            .0
            .as_slice(),
        state.public_key()
    );
    assert_eq!(state.peer_identity().hub(), hub);
    UnparsedPublicKey::new(&ED25519, &key)
        .verify(MESSAGE, signature.as_ref())
        .unwrap();
    assert!(
        UnparsedPublicKey::new(&ED25519, &key)
            .verify(CHANGED_MESSAGE, signature.as_ref())
            .is_err()
    );
    assert_eq!(
        state.socket_path(),
        HubState::socket_path_for(&path).unwrap()
    );
    assert!(!state.socket_path().starts_with(&path));
    drop(state);
    let state = HubState::open(&path, &host).unwrap();
    assert_eq!(state.hub_id(), hub);
    assert_eq!(state.public_key(), key);
}

#[test]
fn state_without_runtime_contract_requires_upgrade_admission() {
    let (_root, path, host) = fixture();
    let state = HubState::open(&path, &host).unwrap();
    drop(state);
    std::fs::remove_file(path.join(crate::constants::STATE_CONTRACT_FILE)).unwrap();
    assert!(HubState::open(&path, &host).is_err());
}

#[test]
fn two_hub_state_directories_cannot_claim_the_same_host_home() {
    let (root, path, host) = fixture();
    let _state = HubState::open(&path, &host).unwrap();
    let other = root.path().canonicalize().unwrap().join(CLONE_DIRECTORY);
    assert!(HubState::open(&other, &host).is_err());
}

#[test]
fn singleton_and_host_binding_prevent_two_local_authorities() {
    let (root, path, host) = fixture();
    let state = HubState::open(&path, &host).unwrap();
    assert!(HubState::open(&path, &host).is_err());
    drop(state);
    assert!(HubState::open(&path, &root.path().join(DIFFERENT_HOST)).is_err());
    assert!(HubState::open(&path, &host).is_ok());
}

#[test]
fn copied_or_replaced_state_requires_explicit_reenrollment() {
    let (root, path, host) = fixture();
    let state = HubState::open(&path, &host).unwrap();
    let record: Record = state
        .namespace
        .read_json(&path.join(HUB_IDENTITY_FILE))
        .unwrap();
    let clone_path = root.path().canonicalize().unwrap().join(CLONE_DIRECTORY);
    let clone = PrivateDirectory::ensure(&clone_path).unwrap();
    let namespace = NamespaceLease::from_directories(vec![(STATE_NAMESPACE, clone)]).unwrap();
    namespace
        .write_json(&clone_path.join(HUB_IDENTITY_FILE), &record)
        .unwrap();
    assert!(HubState::open(&clone_path, &host).is_err());
    std::fs::rename(&path, root.path().join(DIFFERENT_HOST)).unwrap();
    PrivateDirectory::ensure(&path).unwrap();
    assert!(state.verify().is_err());
    assert!(state.sign(MESSAGE).is_err());
    assert!(state.directory().is_err());
}

#[test]
fn invalid_schema_or_key_never_regenerates_a_new_identity() {
    for invalid_schema in [true, false] {
        let (_root, path, host) = fixture();
        let state = HubState::open(&path, &host).unwrap();
        let identity_path = path.join(HUB_IDENTITY_FILE);
        let mut record: Record = state.namespace.read_json(&identity_path).unwrap();
        let hub = record.hub_id;
        if invalid_schema {
            record.schema += 1;
        } else {
            record.key = INVALID_KEY_BYTES.to_vec();
        }
        state.namespace.write_json(&identity_path, &record).unwrap();
        drop(state);
        assert!(HubState::open(&path, &host).is_err());
        let namespace = NamespaceLease::capture(&[(STATE_NAMESPACE, &path)]).unwrap();
        assert_eq!(
            namespace
                .read_json::<Record>(&identity_path)
                .unwrap()
                .hub_id,
            hub
        );
    }
}

#[test]
fn symlinked_identity_and_relative_paths_are_rejected() {
    let (root, path, host) = fixture();
    assert!(HubState::open(Path::new(STATE_DIRECTORY), &host).is_err());
    let directory = PrivateDirectory::ensure(&path).unwrap();
    std::os::unix::fs::symlink(
        root.path().join(CLONE_DIRECTORY),
        path.join(HUB_IDENTITY_FILE),
    )
    .unwrap();
    assert!(HubState::open(directory.path(), &host).is_err());
}

fn rewrite_contract(state: &HubState, path: &Path, update: impl FnOnce(&mut serde_json::Value)) {
    let file = path.join(crate::constants::STATE_CONTRACT_FILE);
    let mut contract: serde_json::Value = state.namespace.read_json(&file).unwrap();
    update(&mut contract);
    state.namespace.write_json(&file, &contract).unwrap();
}

#[test]
fn older_protocol_contract_is_validated_read_only_and_migrated_on_open() {
    let (_root, path, host) = fixture();
    let state = HubState::open(&path, &host).unwrap();
    let current = yolo_hub_rpc_protocol::HUB_PROTOCOL_VERSION;
    rewrite_contract(&state, &path, |contract| {
        contract[crate::constants::HUB_PROTOCOL_FIELD] = serde_json::json!(u16::MIN);
    });
    let file = path.join(crate::constants::STATE_CONTRACT_FILE);
    let before = std::fs::read(&file).unwrap();
    drop(state);
    validate_runtime_contract(&path).unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), before);
    let state = HubState::open(&path, &host).unwrap();
    let contract: serde_json::Value = state.namespace.read_json(&file).unwrap();
    assert_eq!(
        contract[crate::constants::HUB_PROTOCOL_FIELD],
        serde_json::json!(current)
    );
}

#[test]
fn newer_protocol_or_state_schema_is_rejected() {
    for (field, value) in [
        (crate::constants::HUB_PROTOCOL_FIELD, u16::MAX),
        ("state_schema", u16::MAX),
    ] {
        let (_root, path, host) = fixture();
        let state = HubState::open(&path, &host).unwrap();
        rewrite_contract(&state, &path, |contract| {
            contract[field] = serde_json::json!(value);
        });
        drop(state);
        assert!(validate_runtime_contract(&path).is_err());
        assert!(HubState::open(&path, &host).is_err());
    }
}
