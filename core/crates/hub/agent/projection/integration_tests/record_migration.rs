use super::constants::{SNAPSHOT_SCHEMA_FIELD, STATE_FILE};
use super::fixtures::{directory, open};
use super::view::view;
use std::fs;

const PREVIOUS_RECORD_SCHEMA: u16 = 4;
const PREVIOUS_SNAPSHOT_SCHEMA: u16 = 2;
const CURRENT_RECORD_SCHEMA: u16 = 5;
const CURRENT_SNAPSHOT_SCHEMA: u16 = 4;

#[test]
fn previous_record_schema_migrates_in_place_without_losing_entries() {
    let (_root, path, hub) = directory();
    let entries = {
        let mut store = open(&path, hub);
        assert!(store.project(&view()).unwrap().is_some());
        store.list().unwrap().len()
    };
    let file = path.join(STATE_FILE);
    let mut record: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    record["schema"] = serde_json::json!(PREVIOUS_RECORD_SCHEMA);
    for entry in record["entries"].as_object_mut().unwrap().values_mut() {
        entry["snapshot"][SNAPSHOT_SCHEMA_FIELD] = serde_json::json!(PREVIOUS_SNAPSHOT_SCHEMA);
    }
    fs::write(&file, serde_json::to_vec(&record).unwrap()).unwrap();

    let store = open(&path, hub);
    assert_eq!(store.list().unwrap().len(), entries);
    let migrated: serde_json::Value =
        serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    assert_eq!(migrated["schema"], CURRENT_RECORD_SCHEMA);
    for entry in migrated["entries"].as_object().unwrap().values() {
        assert_eq!(
            entry["snapshot"][SNAPSHOT_SCHEMA_FIELD],
            CURRENT_SNAPSHOT_SCHEMA
        );
    }
    drop(store);
    let _reopened = open(&path, hub);
}
