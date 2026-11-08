use crate::support::*;
use yolo_tui_tab::*;

#[test]
fn reopening_restores_exact_recursive_layout_focus_and_remote_addresses() {
    let (_root, path, hub) = fixture();
    let mut store = TabStore::open(&path, hub).unwrap();
    let tab = LocalTabView {
        layout: layout(),
        view: TabViewMode::Terminal,
    };
    store.save(&tab).unwrap();
    let expected = store.document().clone();
    drop(store);
    let store = TabStore::open(&path, hub).unwrap();
    assert_eq!(store.document(), &expected);
    assert_eq!(
        store.document().tabs.active().view.layout,
        tab.layout
    );
}

#[test]
fn retained_directory_matches_the_tab_store_owner_and_survives_independent_handle_drop() {
    let (_root, path, hub) = fixture();
    let store = TabStore::open(&path, hub).unwrap();
    let retained = store.directory().unwrap();
    assert!(retained.same_identity(&yolo_paths::directory::PrivateDirectory::open(&path).unwrap()));
    drop(retained);
    store.directory().unwrap().verify_path().unwrap();
}

#[test]
fn concurrent_owners_and_a_different_entry_hub_cannot_overwrite_the_tab_document() {
    let (_root, path, hub) = fixture();
    let store = TabStore::open(&path, hub).unwrap();
    assert!(TabStore::open(&path, hub).is_err());
    drop(store);
    assert!(TabStore::open(&path, yolo_hub_identity::HubId::new()).is_err());
    assert!(TabStore::open(&path, hub).is_ok());
}

#[test]
fn truncated_and_unknown_field_records_are_not_replaced_with_defaults() {
    let (_root, path, hub) = fixture();
    let store = TabStore::open(&path, hub).unwrap();
    let mut document = serde_json::to_value(store.document()).unwrap();
    drop(store);
    document[INJECTED_FIELD] = true.into();
    yolo_paths::atomic_write_json(&path.join(TAB_FILE), &document).unwrap();
    assert!(TabStore::open(&path, hub).is_err());
    std::fs::write(path.join(TAB_FILE), INVALID_BYTES).unwrap();
    assert!(TabStore::open(&path, hub).is_err());
    assert_eq!(
        std::fs::read(path.join(TAB_FILE)).unwrap(),
        INVALID_BYTES
    );
}
const INJECTED_FIELD: &str = "injected";

#[test]
fn exhausted_revision_never_wraps_or_overwrites_the_last_tab_document() {
    let (_root, path, hub) = fixture();
    let store = TabStore::open(&path, hub).unwrap();
    let mut document = store.document().clone();
    drop(store);
    document.revision = u64::MAX;
    yolo_paths::atomic_write_json(&path.join(TAB_FILE), &document).unwrap();
    let mut store = TabStore::open(&path, hub).unwrap();
    assert!(
        store
            .save(&LocalTabView {
                layout: layout(),
                view: TabViewMode::Terminal
            })
            .is_err()
    );
    let current: TabDocument = yolo_paths::read_json(&path.join(TAB_FILE)).unwrap();
    assert_eq!(current, document);
}
