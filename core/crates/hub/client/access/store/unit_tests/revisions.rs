use super::{constants::*, fixture::*, model::*};
use yolo_hub_routing::Right;

#[test]
fn exact_successor_replaces_the_complete_grant_and_can_remove_every_client() {
    let fixture = Fixture::new();
    let mut store = fixture.open(Some(configuration())).unwrap();
    let original = store.current().unwrap();
    let mut proposed = next(&original);
    proposed.clients[FIRST_CLIENT].origins[FIRST_CLIENT].rights = rights(Right::TerminalWrite);
    assert_eq!(
        store.replace(original.revision, proposed.clone()).unwrap(),
        proposed
    );
    assert_eq!(store.current().unwrap(), proposed);
    let mut empty = next(&proposed);
    empty.clients.clear();
    assert_eq!(
        store.replace(proposed.revision, empty.clone()).unwrap(),
        empty
    );
    drop(store);
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), empty);
}

#[test]
fn cas_revision_and_entry_conflicts_poison_without_overwriting_storage() {
    let mutations: &[fn(&mut u64, &mut yolo_hub_client_access_protocol::AccessConfiguration)] = &[
        |expected, _| *expected = u64::MIN,
        |expected, _| *expected += REVISION_STEP,
        |_, value| value.revision -= REVISION_STEP,
        |_, value| value.revision += REVISION_GAP,
        |_, value| value.entry_hub = other_hub(),
    ];
    for change in mutations {
        let fixture = Fixture::new();
        let mut store = fixture.open(Some(configuration())).unwrap();
        let original = store.current().unwrap();
        let bytes = fixture.bytes();
        let mut expected = original.revision;
        let mut proposed = next(&original);
        change(&mut expected, &mut proposed);
        assert!(store.replace(expected, proposed).is_err());
        assert_poisoned(&mut store, &original);
        assert_eq!(fixture.bytes(), bytes);
        drop(store);
        assert_eq!(fixture.open(None).unwrap().current().unwrap(), original);
    }
}

#[test]
fn invalid_proposals_poison_even_when_followed_by_a_valid_successor() {
    let mutations: &[fn(&mut yolo_hub_client_access_protocol::AccessConfiguration)] = &[
        |value| value.schema_version = FUTURE_SCHEMA,
        |value| value.clients[FIRST_CLIENT].origins.clear(),
        |value| value.clients[FIRST_CLIENT].expires_at_ms = u64::MIN,
        |value| value.clients.push(value.clients[FIRST_CLIENT].clone()),
    ];
    for change in mutations {
        let fixture = Fixture::new();
        let mut store = fixture.open(Some(configuration())).unwrap();
        let original = store.current().unwrap();
        let bytes = fixture.bytes();
        let mut proposed = next(&original);
        change(&mut proposed);
        assert!(store.replace(original.revision, proposed).is_err());
        assert_poisoned(&mut store, &original);
        assert_eq!(fixture.bytes(), bytes);
    }
}

#[test]
fn revision_exhaustion_cannot_wrap_or_restore_the_poisoned_owner() {
    let fixture = Fixture::new();
    let mut original = configuration();
    original.revision = u64::MAX;
    let mut store = fixture.open(Some(original.clone())).unwrap();
    let bytes = fixture.bytes();
    assert!(store.replace(u64::MAX, original.clone()).is_err());
    assert_poisoned(&mut store, &original);
    assert_eq!(fixture.bytes(), bytes);
    drop(store);
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), original);
}
