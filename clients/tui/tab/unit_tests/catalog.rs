use super::support::layout;
use crate::*;

#[test]
fn switching_preserves_each_layout_and_removal_never_reassigns_identity() {
    let mut tabs = TabCatalog::default();
    let first = tabs.active().id;
    let original = LocalTabView {
        layout: layout(),
        view: TabViewMode::Terminal,
    };
    tabs.update_active(original.clone());
    let second = tabs.create(SECOND).unwrap();
    assert_ne!(first, second);
    assert_eq!(tabs.active().view.view, TabViewMode::Inventory);
    assert!(tabs.select(first).unwrap());
    assert!(!tabs.select(first).unwrap());
    assert_eq!(tabs.active().view, original);
    tabs.cycle(false);
    assert_eq!(tabs.active().id, second);
    tabs.cycle(true);
    assert_eq!(tabs.active().id, first);
    tabs.rename(first, RENAMED).unwrap();
    tabs.remove(second).unwrap();
    assert_eq!(tabs.active().name, RENAMED);
    assert!(tabs.remove(first).is_err());
    let second = tabs.create(SECOND).unwrap();
    tabs.remove(second).unwrap();
    assert_eq!(tabs.active().id, first);
    tabs.cycle(true);
    tabs.cycle(false);
    assert_eq!(tabs.active().view, original);
}

#[test]
fn failed_mutations_are_atomic_and_names_cannot_inject_terminal_controls() {
    let mut tabs = TabCatalog::default();
    let first = tabs.active().id;
    tabs.create(SECOND).unwrap();
    for invalid in [
        "",
        " padded",
        "padded ",
        "line\nfeed",
        "escape\x1b",
        &"界".repeat(MAX_TAB_NAME_BYTES),
    ] {
        let before = tabs.clone();
        assert!(tabs.create(invalid).is_err());
        assert!(tabs.rename(first, invalid).is_err());
        assert_eq!(tabs, before);
    }
    let before = tabs.clone();
    assert!(tabs.create(SECOND).is_err());
    assert!(tabs.rename(first, SECOND).is_err());
    let unknown = uuid::Uuid::new_v4();
    assert!(tabs.select(unknown).is_err());
    assert!(tabs.rename(unknown, RENAMED).is_err());
    assert!(tabs.remove(unknown).is_err());
    assert_eq!(tabs, before);
    tabs.rename(first, DEFAULT_TAB_NAME).unwrap();
    tabs
        .rename(first, &"a".repeat(MAX_TAB_NAME_BYTES))
        .unwrap();
}

#[test]
fn bounded_catalog_roundtrips_and_cannot_drop_views_when_full() {
    let mut tabs = TabCatalog::default();
    for index in 1..MAX_TABS {
        tabs.create(&format!("{SECOND}{index}")).unwrap();
    }
    let before = tabs.clone();
    assert!(tabs.create(RENAMED).is_err());
    assert_eq!(tabs, before);
    let encoded = serde_json::to_vec(&tabs).unwrap();
    assert_eq!(
        serde_json::from_slice::<TabCatalog>(&encoded).unwrap(),
        tabs
    );
}

const SECOND: &str = "Remote work";
const RENAMED: &str = "项目";

#[test]
fn document_panes_are_local_to_the_tab_catalog_and_remove_cleanly() {
    let mut catalog = TabCatalog::default();
    let before = catalog.active_layout().panes().len();
    assert!(catalog.open_document("Plugin document".into(), "hello".into()));
    assert_eq!(catalog.active_layout().panes().len(), before);
    assert_eq!(catalog.active_document().unwrap().text, "hello");
    assert!(catalog.close_document());
    assert!(catalog.active_document().is_none());
    assert!(!catalog.close_document());
}

#[test]
fn document_content_persists_with_local_tab_state() {
    let mut catalog = TabCatalog::default();
    assert!(catalog.open_document("Plugin document".into(), "hello".into()));
    let encoded = serde_json::to_vec(&catalog).unwrap();
    assert!(String::from_utf8_lossy(&encoded).contains("Plugin document"));
    let restored: TabCatalog = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored.active_document().unwrap().text, "hello");
}
