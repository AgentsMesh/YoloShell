use super::support::*;
use yolo_hub_connector_store::Store;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_identity::HubId;
use yolo_paths::{directory::PrivateDirectory, namespace_lease::NamespaceLease};

#[test]
fn full_peer_identity_and_directory_birth_identity_prevent_cross_hub_or_clone_adoption() {
    let fixture = Fixture::new();
    let store = fixture.open().unwrap();
    drop(store);
    for peer in [
        PeerIdentity::new(HubId::new(), *fixture.identity.key()).unwrap(),
        PeerIdentity::new(
            fixture.identity.hub(),
            [OTHER_KEY_BYTE; ED25519_PUBLIC_KEY_BYTES],
        )
        .unwrap(),
    ] {
        assert!(Store::<State>::open(fixture.directory(), peer).is_err());
    }
    let clone = fixture.sibling(CLONE_DIRECTORY);
    let directory = PrivateDirectory::ensure(&clone).unwrap();
    PrivateDirectory::ensure(&clone.join(MEMBERS_DIRECTORY)).unwrap();
    let namespace = NamespaceLease::from_directories(vec![(TEST_NAMESPACE, directory)]).unwrap();
    namespace
        .write_json(&clone.join(INDEX_FILE), &fixture.read(&fixture.index()))
        .unwrap();
    assert!(
        Store::<State>::open(PrivateDirectory::open(&clone).unwrap(), fixture.identity).is_err()
    );
}

#[test]
fn root_or_member_directory_replacement_invalidates_the_retained_namespace() {
    for members in [false, true] {
        let fixture = Fixture::new();
        let store = fixture.open().unwrap();
        let replaced = if members {
            fixture.path.join(MEMBERS_DIRECTORY)
        } else {
            fixture.path.clone()
        };
        std::fs::rename(&replaced, fixture.sibling(MOVED_DIRECTORY)).unwrap();
        PrivateDirectory::ensure(&replaced).unwrap();
        assert!(store.verify().is_err());
        assert!(store.entries().is_err());
    }
}

#[test]
fn replaced_lock_never_allows_the_old_owner_to_publish_or_clear_its_poison() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let seed = fixture.seed();
    store.insert(seed.clone(), State::initial(&seed)).unwrap();
    let before = fixture.read(&fixture.member(&seed));
    let lock = fixture.path.join(LOCK_FILE);
    std::fs::remove_file(&lock).unwrap();
    fixture.write(&lock, &());
    assert!(store.verify().is_err());
    assert!(
        store
            .update(seed.connector_id, State::revoked(&seed))
            .is_err()
    );
    assert_eq!(fixture.read(&fixture.member(&seed)), before);
}

#[test]
fn harmless_staging_residue_is_not_adopted_as_a_connector() {
    let fixture = Fixture::new();
    let store = fixture.open().unwrap();
    fixture.write(
        &fixture.path.join(MEMBERS_DIRECTORY).join(RESIDUE_FILE),
        &(),
    );
    assert!(store.entries().unwrap().is_empty());
    store.verify().unwrap();
}
