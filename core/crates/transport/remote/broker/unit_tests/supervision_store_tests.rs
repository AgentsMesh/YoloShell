use super::*;
use serde::{Deserialize, Serialize};
use std::fs;

const ENTRY_NAME: &str = "broker-supervision-test.json";
const REPLACEMENT_NAME: &str = "replacement";
const VALUE: &str = "retained";

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Document {
    value: String,
}

#[test]
fn store_uses_the_expected_retained_directory() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let paths = BrokerPaths::new(&state).unwrap();
    let anchor = paths.state_anchor();
    let store = BrokerSupervisionStore::open_expected(&state, anchor).unwrap();
    let value = Document {
        value: VALUE.into(),
    };
    store.write(ENTRY_NAME, &value).unwrap();
    assert_eq!(store.read::<Document>(ENTRY_NAME).unwrap(), value);

    let displaced = root.path().join(REPLACEMENT_NAME);
    fs::rename(&state, &displaced).unwrap();
    let replacement = BrokerPaths::new(&state).unwrap();
    assert!(store.write(ENTRY_NAME, &value).is_err());
    assert!(!replacement.state_dir().join(ENTRY_NAME).exists());
    assert!(store.read::<Document>(ENTRY_NAME).is_err());
    assert!(store.remove(ENTRY_NAME).is_err());
}

#[test]
fn store_rejects_non_entry_names_and_wrong_anchors() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let paths = BrokerPaths::new(&state).unwrap();
    let store = BrokerSupervisionStore::open_expected(&state, paths.state_anchor()).unwrap();
    let value = Document {
        value: VALUE.into(),
    };
    assert!(store.write("../outside", &value).is_err());

    let displaced = root.path().join(REPLACEMENT_NAME);
    fs::rename(&state, displaced).unwrap();
    BrokerPaths::new(&state).unwrap();
    assert!(BrokerSupervisionStore::open_expected(&state, paths.state_anchor()).is_err());
}
