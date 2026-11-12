use crate::*;
use uuid::Uuid;
use yolo_presentation_layout::Axis;
use yolo_presentation_protocol::{LayoutAddress, LayoutUpdate};

fn document(catalog: &TabCatalog) -> yolo_presentation_protocol::SharedLayoutDocument {
    LayoutUpdate {
        address: LayoutAddress {
            hub_id: yolo_hub_identity::HubId::new(),
            layout_id: Uuid::new_v4(),
        },
        request_id: Uuid::new_v4(),
        expected_revision: 0,
        tabs: catalog.shared_tabs().unwrap(),
    }
    .apply(None)
    .unwrap()
}

#[test]
fn remote_topology_preserves_focus_active_tab_and_local_view() {
    let mut local = TabCatalog::default();
    let tab_id = local.create("second").unwrap();
    let mut tab_view = local.active().view.clone();
    let focused = tab_view.layout.split(Axis::Vertical, None).unwrap();
    tab_view.layout.focus(focused).unwrap();
    tab_view.view = TabViewMode::Inventory;
    local.update_active(tab_view);
    let mut remote = document(&local);
    remote.tabs[0].title = "changed remotely".into();
    let reconciled = TabCatalog::from_shared(&remote, &local).unwrap();
    assert_eq!(reconciled.active().id, tab_id);
    assert_eq!(reconciled.active().view.layout.focused(), focused);
    assert_eq!(reconciled.active().view.view, TabViewMode::Inventory);
    assert_eq!(reconciled.shared_tabs().unwrap(), remote.tabs);
}

#[test]
fn document_panes_survive_topology_adoption_without_changing_it() {
    let mut local = TabCatalog::default();
    let before = local.active_layout().panes().len();
    assert!(local.open_document("doc".into(), "body".into()));
    let remote = document(&local);
    assert_eq!(remote.tabs[0].root.panes().len(), before);
    let reconciled = TabCatalog::from_shared(&remote, &local).unwrap();
    assert_eq!(reconciled.active_document().unwrap().text, "body");
}

#[test]
fn closed_selection_falls_back_to_surviving_identity_without_runtime_actions() {
    let mut local = TabCatalog::default();
    local.create("removed").unwrap();
    let mut remote = document(&local);
    remote.tabs.pop();
    let reconciled = TabCatalog::from_shared(&remote, &local).unwrap();
    assert_eq!(reconciled.active().id, remote.tabs[0].id);
    remote.tabs[0].root = yolo_presentation_layout::Layout::default()
        .document()
        .root
        .clone();
    let replaced = TabCatalog::from_shared(&remote, &reconciled).unwrap();
    assert_eq!(
        replaced.active().view.layout.focused(),
        remote.tabs[0].root.panes()[0].id
    );
}

#[test]
fn empty_remote_document_has_only_a_local_placeholder_and_invalid_documents_fail() {
    let local = TabCatalog::default();
    let mut remote = document(&local);
    remote.tabs.clear();
    let placeholder = TabCatalog::from_shared(&remote, &local).unwrap();
    assert!(
        placeholder
            .active()
            .view
            .layout
            .panes()
            .iter()
            .all(|pane| pane.terminal.is_none())
    );
    assert!(remote.tabs.is_empty());
    remote.revision = 0;
    assert!(TabCatalog::from_shared(&remote, &local).is_err());
}

#[test]
fn shared_title_rules_allow_distinct_tabs_with_identical_titles() {
    let mut local = TabCatalog::default();
    local.create("second").unwrap();
    let mut remote = document(&local);
    remote.tabs[1].title = remote.tabs[0].title.clone();
    let imported = TabCatalog::from_shared(&remote, &TabCatalog::default()).unwrap();
    assert_eq!(imported.shared_tabs().unwrap(), remote.tabs);
}

#[test]
fn selected_layout_identity_is_durable_and_does_not_replace_client_focus() {
    let (_root, path, hub) = super::support::fixture();
    let mut store = TabStore::open(&path, hub).unwrap();
    let before = store.document().tabs.clone();
    let id = Uuid::new_v4();
    store.select_layout(id).unwrap();
    assert!(store.select_layout(Uuid::nil()).is_err());
    drop(store);
    let store = TabStore::open(&path, hub).unwrap();
    assert_eq!(store.document().layout_address.layout_id, id);
    assert_eq!(store.document().tabs, before);
}
