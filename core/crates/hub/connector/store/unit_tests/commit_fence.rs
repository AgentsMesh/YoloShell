use super::support::*;
use crate::{Seed, Store, StoredState};
use serde::{Deserialize, Serialize, Serializer};
use std::cell::RefCell;

thread_local! {
    static BEFORE_SERIALIZE: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(transparent)]
struct FencedState(State);
impl Serialize for FencedState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if let Some(hook) = BEFORE_SERIALIZE.with(|hook| hook.borrow_mut().take()) {
            hook();
        }
        self.0.serialize(serializer)
    }
}
impl StoredState for FencedState {
    fn validate(&self, seed: &Seed) -> anyhow::Result<()> {
        self.0.validate(seed)
    }
}
struct HookReset;
impl Drop for HookReset {
    fn drop(&mut self) {
        BEFORE_SERIALIZE.with(|hook| hook.borrow_mut().take());
    }
}

#[derive(Clone, Copy)]
enum Replacement {
    Index,
    Member,
    Lock,
}

#[test]
fn replacing_a_verified_index_member_or_lock_during_serialization_cannot_commit() {
    for replacement in [Replacement::Index, Replacement::Member, Replacement::Lock] {
        let fixture = Fixture::new();
        let mut store = Store::<FencedState>::open(fixture.directory(), fixture.identity).unwrap();
        let seed = fixture.seed();
        store
            .insert(seed.clone(), FencedState(State::initial(&seed)))
            .unwrap();
        let original = fixture.read(&fixture.member(&seed));
        let path = match replacement {
            Replacement::Index => fixture.index(),
            Replacement::Member => fixture.member(&seed),
            Replacement::Lock => fixture.path.join(LOCK_FILE),
        };
        let value = match replacement {
            Replacement::Lock => serde_json::Value::Null,
            _ => fixture.read(&path),
        };
        let namespace = fixture.namespace();
        BEFORE_SERIALIZE.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move || {
                if matches!(replacement, Replacement::Lock) {
                    std::fs::remove_file(&path).unwrap();
                }
                namespace.write_json(&path, &value).unwrap();
            }))
        });
        let _reset = HookReset;
        assert!(
            store
                .update(seed.connector_id, FencedState(State::revoked(&seed)))
                .is_err()
        );
        assert!(store.verify().is_err());
        assert_eq!(fixture.read(&fixture.member(&seed)), original);
    }
}
