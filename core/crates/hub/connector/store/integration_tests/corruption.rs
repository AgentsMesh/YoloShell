use super::support::*;

#[test]
fn replaced_index_or_member_poison_the_owner_even_when_contents_are_identical() {
    for index in [true, false] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        let seed = fixture.seed();
        store.insert(seed.clone(), State::initial(&seed)).unwrap();
        let path = if index {
            fixture.index()
        } else {
            fixture.member(&seed)
        };
        let previous = fixture.read(&path);
        fixture.write(&path, &previous);
        assert!(store.get(seed.connector_id).is_err());
        assert!(
            store
                .update(seed.connector_id, State::revoked(&seed))
                .is_err()
        );
        assert!(store.entries().is_err());
        assert_eq!(fixture.read(&path), previous);
        drop(store);
        fixture.open().unwrap().verify().unwrap();
    }
}

#[test]
fn ready_missing_member_and_unindexed_member_are_not_recovered_as_new_enrollments() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let seed = fixture.seed();
    store.insert(seed.clone(), State::revoked(&seed)).unwrap();
    std::fs::remove_file(fixture.member(&seed)).unwrap();
    assert!(store.get(seed.connector_id).is_err());
    drop(store);
    assert!(fixture.open().is_err());
    assert!(!fixture.member(&seed).exists());

    let fixture = Fixture::new();
    let store = fixture.open().unwrap();
    let rogue = fixture.seed();
    fixture.replace_member(&rogue, &State::initial(&rogue));
    assert!(store.get(rogue.connector_id).is_err());
    drop(store);
    assert!(fixture.open().is_err());
    std::fs::remove_file(fixture.index()).unwrap();
    assert!(fixture.open().is_err());
    assert!(!fixture.index().exists());
}

#[test]
fn modified_seed_or_invalid_mutable_state_is_rejected_after_reopen() {
    for seed_changed in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        let seed = fixture.seed();
        store.insert(seed.clone(), State::initial(&seed)).unwrap();
        drop(store);
        let mut changed = seed.clone();
        let mut state = State::initial(&seed);
        if seed_changed {
            changed.credential = yolo_hub_connector_protocol::ConnectorCredential::generate();
        } else {
            state.revision = INVALID_REVISION;
        }
        fixture.replace_member(&changed, &state);
        assert!(fixture.open().is_err());
    }
}

#[test]
fn malformed_member_or_index_never_gets_overwritten_by_initialization() {
    for index in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        let seed = fixture.seed();
        store.insert(seed.clone(), State::initial(&seed)).unwrap();
        drop(store);
        let path = if index {
            fixture.index()
        } else {
            fixture.member(&seed)
        };
        std::fs::write(&path, INVALID_DOCUMENT).unwrap();
        assert!(fixture.open().is_err());
        assert_eq!(std::fs::read(path).unwrap(), INVALID_DOCUMENT);
    }
}

#[test]
fn symlinked_member_never_reads_or_rewrites_the_foreign_target() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let seed = fixture.seed();
    store.insert(seed.clone(), State::initial(&seed)).unwrap();
    let foreign = fixture.sibling(MOVED_DIRECTORY);
    std::fs::rename(fixture.member(&seed), &foreign).unwrap();
    let bytes = std::fs::read(&foreign).unwrap();
    std::os::unix::fs::symlink(&foreign, fixture.member(&seed)).unwrap();
    assert!(store.get(seed.connector_id).is_err());
    assert!(
        store
            .update(seed.connector_id, State::revoked(&seed))
            .is_err()
    );
    drop(store);
    assert!(fixture.open().is_err());
    assert_eq!(std::fs::read(foreign).unwrap(), bytes);
}
