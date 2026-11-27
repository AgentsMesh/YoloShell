use super::fixtures::{constants::*, *};
use crate::constants::*;

#[test]
fn corrupt_record_schema_revision_change_or_configuration_is_not_reinitialized() {
    let mutations: &[fn(&mut crate::record::Record)] = &[
        |r| r.state.schema_version += SCHEMA_STEP,
        |r| r.state.revision = 0,
        |r| r.state.last_change = uuid::Uuid::nil(),
        |r| r.state.configuration = invalid_configuration(r.state.identity),
        |r| r.state.identity = other_identity(),
    ];
    for mutate in mutations {
        let fixture = Fixture::new();
        let store = fixture.open(None).unwrap();
        let mut record = fixture.record();
        drop(store);
        mutate(&mut record);
        fixture.replace(&record);
        let before = fixture.bytes();
        assert!(fixture.open(None).is_err());
        assert_eq!(fixture.bytes(), before);
    }
}

#[test]
fn unknown_fields_and_invalid_json_fail_without_replacement() {
    for nested in [false, true] {
        let fixture = Fixture::new();
        let store = fixture.open(None).unwrap();
        let mut value = serde_json::to_value(fixture.record()).unwrap();
        drop(store);
        if nested {
            value[STATE_FIELD][UNKNOWN_FIELD] = UNKNOWN_VALUE.into();
        } else {
            value[UNKNOWN_FIELD] = UNKNOWN_VALUE.into();
        }
        fixture.replace(&value);
        let before = fixture.bytes();
        assert!(fixture.open(None).is_err());
        assert_eq!(fixture.bytes(), before);
    }
    let fixture = Fixture::new();
    let store = fixture.open(None).unwrap();
    drop(store);
    std::fs::write(fixture.path.join(RECORD_FILE), INVALID_BYTES).unwrap();
    assert!(fixture.open(None).is_err());
    assert_eq!(fixture.bytes(), INVALID_BYTES);
}

#[test]
fn a_failed_read_permanently_marks_the_handle_lost() {
    let fixture = Fixture::new();
    let store = fixture.open(None).unwrap();
    assert!(!store.is_lost());
    std::fs::remove_file(fixture.path.join(RECORD_FILE)).unwrap();
    assert!(store.current().is_err());
    assert!(store.is_lost());
    assert_eq!(store.current().unwrap_err().to_string(), LOST);
}

#[test]
fn even_valid_file_replacement_invalidates_loaded_identity_and_prevents_automatic_adoption() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let request = change(&store.current().unwrap());
    let record = fixture.record();
    fixture.replace(&record);
    let bytes = fixture.bytes();
    assert_eq!(store.current().unwrap_err().to_string(), REPLACED_RECORD);
    assert_eq!(store.current().unwrap_err().to_string(), LOST);
    assert!(store.apply(request).is_err());
    assert_eq!(fixture.bytes(), bytes);
}
