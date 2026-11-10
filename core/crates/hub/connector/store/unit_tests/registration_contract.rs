use super::support::*;
use crate::{
    REGISTRATION_PHASE_TAG,
    record::{Member, Record},
};
use serde_json::Value;

const MEMBERS_FIELD: &str = "members";
const INITIAL_FIELD: &str = "initial";
const PREPARING: &str = "preparing";
const READY: &str = "ready";

#[test]
fn registration_intent_is_durable_before_member_and_ready_compacts_the_initial_state() {
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
    let preparing = only_member(&fixture);
    assert_eq!(preparing[REGISTRATION_PHASE_TAG], PREPARING);
    assert_eq!(
        preparing[INITIAL_FIELD][STATE_FIELD],
        serde_json::to_value(&state).unwrap()
    );
    assert!(!fixture.member(&seed).exists());
    assert!(preparing.get(RETIRED_PHASE_FIELD).is_none());
    store.stage_member(seed.connector_id).unwrap();
    store.finish(seed.connector_id).unwrap();
    let ready = only_member(&fixture);
    assert_eq!(ready[REGISTRATION_PHASE_TAG], READY);
    assert!(ready.get(INITIAL_FIELD).is_none());
    assert!(ready.get(DIGEST_FIELD).is_some());
    assert_eq!(
        store.get(seed.connector_id).unwrap(),
        Some((seed.clone(), state.clone()))
    );
    for mut obsolete in [preparing, ready] {
        let fields = obsolete.as_object_mut().unwrap();
        let phase = fields.remove(REGISTRATION_PHASE_TAG).unwrap();
        fields.insert(RETIRED_PHASE_FIELD.into(), phase);
        assert!(serde_json::from_value::<Member<State>>(obsolete).is_err());
    }
    drop(store);
    assert_eq!(
        fixture.open().unwrap().get(seed.connector_id).unwrap(),
        Some((seed, state))
    );
}

#[test]
fn a_member_created_after_preparation_is_never_overwritten_or_implicitly_adopted() {
    for exact_initial in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        let seed = fixture.seed();
        let initial = State::initial(&seed);
        store
            .prepare(Record {
                seed: seed.clone(),
                state: initial.clone(),
            })
            .unwrap();
        let written = if exact_initial {
            initial.clone()
        } else {
            State::revoked(&seed)
        };
        fixture.replace_member(&seed, &written);
        let index = fixture.file_identity(&fixture.index());
        let member = fixture.file_identity(&fixture.member(&seed));
        assert!(store.stage_member(seed.connector_id).is_err());
        assert_eq!(fixture.file_identity(&fixture.index()), index);
        assert_eq!(fixture.file_identity(&fixture.member(&seed)), member);
        drop(store);
        let reopened = fixture.open();
        if exact_initial {
            assert_eq!(
                reopened.unwrap().get(seed.connector_id).unwrap(),
                Some((seed, initial))
            );
        } else {
            assert!(reopened.is_err());
            assert_eq!(fixture.file_identity(&fixture.member(&seed)), member);
        }
    }
}

fn only_member(fixture: &Fixture) -> Value {
    let index = fixture.read(&fixture.index());
    let [member] = index[MEMBERS_FIELD].as_array().unwrap().as_slice() else {
        panic!()
    };
    member.clone()
}
