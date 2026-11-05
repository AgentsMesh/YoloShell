use crate::{constants::*, records, support::Fixture};
use yolo_hub_configuration_store::RevisionConflict;
use yolo_hub_management_protocol::{CONFIGURATION_REVISION_STEP, ConfigurationChange};

#[test]
fn check_is_read_only_and_successful_compare_and_swap_increments_exactly_once() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None);
    let current = store.current().unwrap();
    let change = records::change(&current);
    assert_eq!(store.check(&change).unwrap(), current);
    assert_eq!(store.current().unwrap(), current);
    let applied = store.apply(change.clone()).unwrap();
    assert_eq!(
        applied.revision,
        current.revision + CONFIGURATION_REVISION_STEP
    );
    assert_eq!(applied.last_change, change.request_id);
    assert_eq!(applied.configuration, change.configuration);
    assert_eq!(applied.identity, current.identity);
    assert_eq!(store.current().unwrap(), applied);
    let stale = ConfigurationChange {
        request_id: uuid::Uuid::new_v4(),
        ..change
    };
    assert!(
        store
            .check(&stale)
            .unwrap_err()
            .downcast_ref::<RevisionConflict>()
            .is_some()
    );
    assert!(
        store
            .apply(stale)
            .unwrap_err()
            .downcast_ref::<RevisionConflict>()
            .is_some()
    );
    assert_eq!(store.current().unwrap(), applied);
    let future = ConfigurationChange {
        expected_revision: applied.revision + REVISION_EXCESS,
        ..records::change(&applied)
    };
    assert!(
        store
            .apply(future)
            .unwrap_err()
            .downcast_ref::<RevisionConflict>()
            .is_some()
    );
    assert!(store.apply(records::change(&applied)).is_ok());
}

#[test]
fn exact_duplicate_returns_original_state_without_a_write_even_after_restart() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None);
    let change = records::change(&store.current().unwrap());
    let applied = store.apply(change.clone()).unwrap();
    let (_, before) = fixture
        .namespace()
        .read_json_with_identity::<serde_json::Value>(&fixture.path.join(RECORD_FILE))
        .unwrap();
    assert_eq!(store.check(&change).unwrap(), applied);
    assert_eq!(store.apply(change.clone()).unwrap(), applied);
    let (_, after) = fixture
        .namespace()
        .read_json_with_identity::<serde_json::Value>(&fixture.path.join(RECORD_FILE))
        .unwrap();
    assert_eq!(before, after);
    drop(store);
    let mut store = fixture.open(None);
    assert_eq!(store.apply(change).unwrap(), applied);
    let (_, reopened) = fixture
        .namespace()
        .read_json_with_identity::<serde_json::Value>(&fixture.path.join(RECORD_FILE))
        .unwrap();
    assert_eq!(before, reopened);
}

#[test]
fn reusing_last_request_with_another_payload_or_precondition_is_a_conflict() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None);
    let change = records::change(&store.current().unwrap());
    let applied = store.apply(change.clone()).unwrap();
    let mutations: &[fn(&mut ConfigurationChange, u64)] = &[
        |change, _| change.configuration = records::configuration(),
        |change, current| change.expected_revision = current,
        |change, _| change.expected_revision = u64::MAX,
    ];
    for mutate in mutations {
        let mut reused = change.clone();
        mutate(&mut reused, applied.revision);
        assert!(
            store
                .apply(reused)
                .unwrap_err()
                .downcast_ref::<RevisionConflict>()
                .is_some()
        );
        assert_eq!(store.current().unwrap(), applied);
    }
    let newer = store.apply(records::change(&applied)).unwrap();
    assert!(
        store
            .apply(change)
            .unwrap_err()
            .downcast_ref::<RevisionConflict>()
            .is_some()
    );
    assert_eq!(store.current().unwrap(), newer);
}

#[test]
fn invalid_change_and_revision_exhaustion_do_not_poison_or_overwrite_storage() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None);
    let current = store.current().unwrap();
    let mutations: &[fn(&mut ConfigurationChange)] = &[
        |change| change.request_id = uuid::Uuid::nil(),
        |change| change.expected_revision = u64::MIN,
    ];
    for mutate in mutations {
        let mut change = records::change(&current);
        mutate(&mut change);
        assert!(store.apply(change).is_err());
        assert_eq!(store.current().unwrap(), current);
    }
    let seed_id = ConfigurationChange {
        request_id: current.last_change,
        expected_revision: current.revision,
        configuration: current.configuration.clone(),
    };
    assert!(
        store
            .apply(seed_id)
            .unwrap_err()
            .downcast_ref::<RevisionConflict>()
            .is_some()
    );
    drop(store);
    let mut record = fixture.read();
    record[STATE_FIELD][REVISION_FIELD] = u64::MAX.into();
    fixture.write(&record);
    let mut store = fixture.open(None);
    let exhausted = store.current().unwrap();
    assert!(store.check(&records::change(&exhausted)).is_err());
    assert!(store.apply(records::change(&exhausted)).is_err());
    assert_eq!(store.current().unwrap(), exhausted);
    assert_eq!(fixture.read(), record);
}
