mod catalog;
mod catalog_validation;
mod machines;
mod shared_layout;
#[path = "../test_support/mod.rs"]
mod support;
use super::*;
use support::*;

#[test]
fn record_rejects_foreign_entry_version_and_zero_revision() {
    let (_root, path, hub) = fixture();
    let store = TabStore::open(&path, hub).unwrap();
    let original = store.document();
    original.validate(hub).unwrap();
    assert!(original.validate(yolo_hub_identity::HubId::new()).is_err());
    let mut changed = original.clone();
    changed.schema_version += TAB_SCHEMA_VERSION;
    assert!(changed.validate(hub).is_err());
    changed = original.clone();
    changed.revision = 0;
    assert!(changed.validate(hub).is_err());
}

#[test]
fn default_locations_are_per_entry_and_reject_relative_base_paths() {
    let (_root, path, hub) = fixture();
    let location = super::location::location(path.clone(), hub).unwrap();
    assert!(location.starts_with(&path));
    assert_eq!(
        location.file_name().unwrap(),
        std::ffi::OsStr::new(&hub.to_string())
    );
    assert!(super::location::location(std::path::PathBuf::from(DIRECTORY), hub).is_err());
    assert_eq!(
        super::location::from_home(Some(path.into_os_string()), hub).unwrap(),
        location
    );
    assert!(super::location::from_home(None, hub).is_err());
    let actual = super::location::default_directory(hub);
    assert_eq!(
        actual.is_ok(),
        std::env::var_os(super::constants::HOME_ENV).is_some()
    );
}

#[test]
fn document_contains_no_runtime_or_writer_authority() {
    let (_root, path, hub) = fixture();
    let mut store = TabStore::open(&path, hub).unwrap();
    let tab = LocalTabView {
        layout: layout(),
        view: TabViewMode::Terminal,
    };
    assert!(store.save(&tab).unwrap());
    assert!(!store.save(&tab).unwrap());
    let value = serde_json::to_value(store.document()).unwrap();
    assert_eq!(value.as_object().unwrap().len(), DOCUMENT_FIELDS);
    assert_eq!(
        serde_json::to_value(&store.document().tabs.active().view)
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        TAB_FIELDS
    );
    assert_eq!(store.document().tabs.active().view, tab);
}

const DOCUMENT_FIELDS: usize = 6;
const TAB_FIELDS: usize = 2;

#[test]
fn client_state_override_is_a_direct_absolute_root_and_does_not_require_home() {
    let (_root, path, hub) = fixture();
    assert_eq!(
        super::location::resolved(Some(path.clone().into_os_string()), None, hub).unwrap(),
        path
    );
    for invalid in ["", "relative", "/", "/tmp/../state"] {
        assert!(
            super::location::resolved(
                Some(invalid.into()),
                Some(path.clone().into_os_string()),
                hub
            )
            .is_err()
        );
    }
    assert_eq!(
        super::location::resolved(None, Some(path.clone().into_os_string()), hub).unwrap(),
        super::location::location(path, hub).unwrap()
    );
}
