use crate::support::*;
use serde_json::{Value, json};
use yolo_tui_tab::*;

#[test]
fn single_workspace_format_is_rejected_without_rewriting_bindings() {
    let (_root, path, hub) = fixture();
    drop(TabStore::open(&path, hub).unwrap());
    let tab = LocalTabView {
        layout: layout(),
        view: TabViewMode::Terminal,
    };
    let unsupported = json!({ SCHEMA_FIELD: UNSUPPORTED_SCHEMA, ENTRY_FIELD: hub, REVISION_FIELD: STORED_REVISION, RETIRED_WORKSPACE_FIELD: tab });
    yolo_paths::atomic_write_json(&path.join(TAB_FILE), &unsupported).unwrap();
    let bytes = std::fs::read(path.join(TAB_FILE)).unwrap();
    assert!(TabStore::open(&path, hub).is_err());
    assert_eq!(std::fs::read(path.join(TAB_FILE)).unwrap(), bytes);
}

#[test]
fn unsupported_records_remain_untouched_including_foreign_owner_and_exhaustion() {
    let (_root, path, hub) = fixture();
    drop(TabStore::open(&path, hub).unwrap());
    let original = json!({ SCHEMA_FIELD: UNSUPPORTED_SCHEMA, ENTRY_FIELD: hub, REVISION_FIELD: FIRST_TAB_REVISION, RETIRED_WORKSPACE_FIELD: LocalTabView::default() });
    for (field, value) in [
        (SCHEMA_FIELD, Value::from(TAB_SCHEMA_VERSION)),
        (REVISION_FIELD, Value::from(0)),
        (REVISION_FIELD, Value::from(u64::MAX)),
        (
            ENTRY_FIELD,
            serde_json::to_value(yolo_hub_identity::HubId::new()).unwrap(),
        ),
        ("unexpected", Value::Bool(true)),
    ] {
        let mut invalid = original.clone();
        invalid[field] = value;
        yolo_paths::atomic_write_json(&path.join(TAB_FILE), &invalid).unwrap();
        assert!(TabStore::open(&path, hub).is_err());
        assert_eq!(
            yolo_paths::read_json::<Value>(&path.join(TAB_FILE)).unwrap(),
            invalid
        );
    }
}

#[test]
fn all_named_layouts_restore_and_conflicts_preserve_the_whole_catalog() {
    let (_root, path, hub) = fixture();
    let mut store = TabStore::open(&path, hub).unwrap();
    let mut tabs = store.document().tabs.clone();
    tabs.update_active(LocalTabView {
        layout: layout(),
        view: TabViewMode::Terminal,
    });
    tabs.create("Remote").unwrap();
    tabs.update_active(LocalTabView {
        layout: layout(),
        view: TabViewMode::Inventory,
    });
    assert!(store.save_catalog(&tabs).unwrap());
    assert!(!store.save_catalog(&tabs).unwrap());
    drop(store);
    let mut store = TabStore::open(&path, hub).unwrap();
    assert_eq!(store.document().tabs, tabs);
    let replacement = store.document().clone();
    yolo_paths::atomic_write_json(&path.join(TAB_FILE), &replacement).unwrap();
    tabs.remove(tabs.entries()[0].id).unwrap();
    assert!(store.save_catalog(&tabs).is_err());
    assert_eq!(
        yolo_paths::read_json::<TabDocument>(&path.join(TAB_FILE)).unwrap(),
        replacement
    );
}
