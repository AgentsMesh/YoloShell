use super::support::*;

#[test]
fn state_updates_touch_only_one_member_and_reopen_preserves_revocation_and_revision() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let first = fixture.seed();
    let second = fixture.seed();
    store.insert(first.clone(), State::initial(&first)).unwrap();
    store
        .insert(second.clone(), State::initial(&second))
        .unwrap();
    let index_identity = fixture.file_identity(&fixture.index());
    let second_identity = fixture.file_identity(&fixture.member(&second));
    let updated = State::revoked(&first);
    store.update(first.connector_id, updated.clone()).unwrap();
    assert_eq!(fixture.file_identity(&fixture.index()), index_identity);
    assert_eq!(
        fixture.file_identity(&fixture.member(&second)),
        second_identity
    );
    assert_eq!(
        store.get(first.connector_id).unwrap(),
        Some((first.clone(), updated.clone()))
    );
    assert!(fixture.open().is_err());
    store.verify().unwrap();
    drop(store);
    let restored = fixture.open().unwrap();
    assert_eq!(
        restored.entries().unwrap(),
        vec![(first, updated), (second.clone(), State::initial(&second))]
    );
    restored.verify().unwrap();
}

#[test]
fn durable_json_limit_rejects_large_member_update_without_partial_publication() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let seed = fixture.seed();
    store.insert(seed.clone(), State::initial(&seed)).unwrap();
    let before = fixture.file_identity(&fixture.member(&seed));
    let mut oversized = State::revoked(&seed);
    oversized.payload = PAYLOAD_BYTE.repeat(yolo_paths::constants::MAX_DURABLE_JSON_BYTES as usize);
    assert!(store.update(seed.connector_id, oversized).is_err());
    assert!(store.get(seed.connector_id).is_err());
    assert_eq!(fixture.file_identity(&fixture.member(&seed)), before);
    drop(store);
    assert_eq!(
        fixture.open().unwrap().get(seed.connector_id).unwrap(),
        Some((seed.clone(), State::initial(&seed)))
    );
}

#[test]
fn oversized_registration_never_publishes_a_partial_intent_or_member() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let seed = fixture.seed();
    let before = fixture.file_identity(&fixture.index());
    let mut state = State::initial(&seed);
    state.payload = PAYLOAD_BYTE.repeat(yolo_paths::constants::MAX_DURABLE_JSON_BYTES as usize);
    assert!(store.insert(seed.clone(), state).is_err());
    assert_eq!(fixture.file_identity(&fixture.index()), before);
    assert!(!fixture.member(&seed).exists());
    drop(store);
    assert!(fixture.open().unwrap().entries().unwrap().is_empty());
}
