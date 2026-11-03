use super::support::*;
use crate::{MAX_CONNECTORS, record::Member};

#[test]
fn invalid_state_or_duplicate_registration_poison_the_instance_without_updating_members() {
    for duplicate_request in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        let first = fixture.seed();
        store.insert(first.clone(), State::initial(&first)).unwrap();
        let before = fixture.read(&fixture.member(&first));
        let mut duplicate = fixture.seed();
        if duplicate_request {
            duplicate.request_id = first.request_id;
        } else {
            duplicate.connector_id = first.connector_id;
        }
        assert!(
            store
                .insert(duplicate.clone(), State::initial(&duplicate))
                .is_err()
        );
        assert!(store.get(first.connector_id).is_err());
        assert!(store.verify().is_err());
        assert_eq!(fixture.read(&fixture.member(&first)), before);
    }
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let seed = fixture.seed();
    store.insert(seed.clone(), State::initial(&seed)).unwrap();
    let mut invalid = State::initial(&seed);
    invalid.revision = INVALID_REVISION;
    assert!(store.update(seed.connector_id, invalid).is_err());
    assert!(store.entries().is_err());
}

#[test]
fn unknown_get_is_absence_but_unknown_update_is_an_error() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let missing = fixture.seed();
    assert!(store.get(missing.connector_id).unwrap().is_none());
    assert!(store.entries().unwrap().is_empty());
    assert!(
        store
            .update(missing.connector_id, State::initial(&missing))
            .is_err()
    );
    assert!(store.verify().is_err());
}

#[test]
fn registration_capacity_is_checked_before_a_new_intent_is_written() {
    let fixture = Fixture::new();
    let store = fixture.open().unwrap();
    let mut index = store.index.clone();
    for _ in 0..MAX_CONNECTORS {
        let seed = fixture.seed();
        index.members.push(Member::Ready {
            connector_id: seed.connector_id,
            request_id: seed.request_id,
            seed_digest: seed.digest().unwrap(),
        });
    }
    let before = fixture.read(&fixture.index());
    let seed = fixture.seed();
    assert_eq!(
        index.require_available_seed(&seed).unwrap_err().to_string(),
        crate::constants::CAPACITY
    );
    assert_eq!(fixture.read(&fixture.index()), before);
    assert!(!fixture.member(&seed).exists());
}
