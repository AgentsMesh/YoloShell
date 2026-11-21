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
        tabs: catalog.shared_tabs(catalog.machine()).unwrap(),
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
    let reconciled = TabCatalog::from_shared(&remote, &local, local.machine()).unwrap();
    assert_eq!(reconciled.active().id, tab_id);
    assert_eq!(reconciled.active().view.layout.focused(), focused);
    assert_eq!(reconciled.active().view.view, TabViewMode::Inventory);
    assert_eq!(reconciled.shared_tabs(reconciled.machine()).unwrap(), remote.tabs);
}

#[test]
fn document_panes_survive_topology_adoption_without_changing_it() {
    let mut local = TabCatalog::default();
    let before = local.active_layout().panes().len();
    assert!(local.open_document("doc".into(), "body".into()));
    let remote = document(&local);
    assert_eq!(remote.tabs[0].root.panes().len(), before);
    let reconciled = TabCatalog::from_shared(&remote, &local, local.machine()).unwrap();
    assert_eq!(reconciled.active_document().unwrap().text, "body");
}

#[test]
fn closed_selection_falls_back_to_surviving_identity_without_runtime_actions() {
    let mut local = TabCatalog::default();
    local.create("removed").unwrap();
    let mut remote = document(&local);
    remote.tabs.pop();
    let reconciled = TabCatalog::from_shared(&remote, &local, local.machine()).unwrap();
    assert_eq!(reconciled.active().id, remote.tabs[0].id);
    remote.tabs[0].root = yolo_presentation_layout::Layout::default()
        .document()
        .root
        .clone();
    let replaced = TabCatalog::from_shared(&remote, &reconciled, reconciled.machine()).unwrap();
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
    let placeholder = TabCatalog::from_shared(&remote, &local, local.machine()).unwrap();
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
    assert!(TabCatalog::from_shared(&remote, &local, local.machine()).is_err());
}

#[test]
fn shared_title_rules_allow_distinct_tabs_with_identical_titles() {
    let mut local = TabCatalog::default();
    local.create("second").unwrap();
    let mut remote = document(&local);
    remote.tabs[1].title = remote.tabs[0].title.clone();
    let entry = local.machine();
    let previous = TabCatalog::new(entry, LocalTabView::default());
    let imported = TabCatalog::from_shared(&remote, &previous, entry).unwrap();
    assert_eq!(imported.shared_tabs(entry).unwrap(), remote.tabs);
}

#[test]
fn adopting_entry_tabs_preserves_remote_machine_workspaces() {
    let entry = yolo_hub_identity::HubId::new();
    let remote_hub = yolo_hub_identity::HubId::new();
    let mut local = TabCatalog::new(entry, LocalTabView::default());
    local.select_machine(remote_hub).unwrap();
    let remote_tab = local.create("remote-work").unwrap();
    local.select_machine(entry).unwrap();
    let mut shared = document(&local);
    shared.tabs[0].title = "entry changed".into();
    let adopted = TabCatalog::from_shared(&shared, &local, entry).unwrap();
    let kept = adopted
        .entries()
        .iter()
        .find(|tab| tab.id == remote_tab)
        .expect("remote tab survives");
    assert_eq!(kept.machine, remote_hub);
    assert_eq!(
        adopted.active_by_machine().get(&remote_hub),
        Some(&remote_tab)
    );
    assert_eq!(adopted.machine(), entry);
    assert_eq!(adopted.shared_tabs(entry).unwrap(), shared.tabs);
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
