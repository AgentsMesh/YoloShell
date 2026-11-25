use crate::*;
use yolo_hub_identity::HubId;

#[test]
fn machines_own_independent_tabs_and_active_selection() {
    let first = HubId::new();
    let second = HubId::new();
    let mut tabs = TabCatalog::new(first, LocalTabView::default());
    let first_id = tabs.active().id;
    assert_eq!(tabs.active().machine, first);
    assert!(tabs.select_machine(second).unwrap());
    let second_id = tabs.active().id;
    assert_ne!(first_id, second_id);
    assert_eq!(tabs.machine(), second);
    assert_eq!(tabs.active().name, DEFAULT_TAB_NAME);
    assert!(tabs.create(DEFAULT_TAB_NAME).is_err());
    let build = tabs.create("build").unwrap();
    assert_eq!(tabs.machine_entries().len(), 2);
    assert!(tabs.select(first_id).unwrap());
    assert_eq!(tabs.machine(), first);
    assert_eq!(tabs.machine_entries().len(), 1);
    assert!(!tabs.select(first_id).unwrap());
    assert!(tabs.select_machine(second).unwrap());
    assert_eq!(tabs.active().id, build);
    tabs.select(first_id).unwrap();
    assert!(tabs.remove(first_id).is_err());
    tabs.select_machine(second).unwrap();
    tabs.create("second").unwrap();
    tabs.select(second_id).unwrap();
    tabs.remove(second_id).unwrap();
    assert_eq!(tabs.active().id, build);
    assert_eq!(tabs.active_by_machine().get(&second), Some(&build));
    assert_eq!(tabs.active_by_machine().get(&first), Some(&first_id));
}

#[test]
fn per_machine_state_roundtrips_through_the_document() {
    let first = HubId::new();
    let second = HubId::new();
    let mut tabs = TabCatalog::new(first, LocalTabView::default());
    tabs.select_machine(second).unwrap();
    tabs.create("remote").unwrap();
    tabs.select_machine(first).unwrap();
    let encoded = serde_json::to_vec(&tabs).unwrap();
    let restored: TabCatalog = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, tabs);
    assert_eq!(restored.machine(), first);
    assert_eq!(restored.active_by_machine().len(), 2);
    assert_eq!(restored.machine_entries().len(), 1);
    restored.shared_tabs(first).unwrap();
    assert!(restored.shared_tabs(HubId::new()).is_err());
}
