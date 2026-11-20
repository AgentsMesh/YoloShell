use super::support::*;
use crate::record::Record;

#[test]
fn preparing_intent_replays_missing_or_exact_initial_member_before_ready() {
    for stage_member in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        let seed = fixture.seed();
        let state = State::initial(&seed);
        store
            .prepare(Record {
                seed: seed.clone(),
                state: state.clone(),
            })
            .unwrap();
        assert!(!fixture.member(&seed).exists());
        if stage_member {
            store.stage_member(seed.connector_id).unwrap();
        }
        drop(store);
        let mut restored = fixture.open().unwrap();
        assert_eq!(
            restored.get(seed.connector_id).unwrap(),
            Some((seed.clone(), state))
        );
        restored
            .update(seed.connector_id, State::revoked(&seed))
            .unwrap();
        drop(restored);
        assert_eq!(
            fixture.open().unwrap().get(seed.connector_id).unwrap(),
            Some((seed.clone(), State::revoked(&seed)))
        );
    }
}

#[test]
fn preparing_cannot_accept_a_member_that_already_changed_state_or_seed() {
    for change_seed in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        let seed = fixture.seed();
        store
            .prepare(Record {
                seed: seed.clone(),
                state: State::initial(&seed),
            })
            .unwrap();
        store.stage_member(seed.connector_id).unwrap();
        let mut record = Record {
            seed: seed.clone(),
            state: State::revoked(&seed),
        };
        if change_seed {
            record.state = State::initial(&seed);
            record.seed.credential = yolo_hub_connector_protocol::ConnectorCredential::generate();
        }
        fixture.write(&fixture.member(&seed), &record);
        drop(store);
        assert!(fixture.open().is_err());
    }
}

#[test]
fn recovery_validates_all_ready_members_before_completing_another_intent() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let ready = fixture.seed();
    store.insert(ready.clone(), State::revoked(&ready)).unwrap();
    let pending = fixture.seed();
    store
        .prepare(Record {
            seed: pending.clone(),
            state: State::initial(&pending),
        })
        .unwrap();
    std::fs::remove_file(fixture.member(&ready)).unwrap();
    drop(store);
    assert!(fixture.open().is_err());
    assert!(!fixture.member(&pending).exists());
}

#[test]
fn phase_helpers_never_restage_ready_members_or_ack_missing_records() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let seed = fixture.seed();
    store.insert(seed.clone(), State::initial(&seed)).unwrap();
    assert!(store.stage_member(seed.connector_id).is_err());
    assert!(store.finish(seed.connector_id).is_err());
    let missing = fixture.seed();
    store
        .prepare(Record {
            seed: missing.clone(),
            state: State::initial(&missing),
        })
        .unwrap();
    assert!(store.finish(missing.connector_id).is_err());
}
