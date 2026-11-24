use super::fixtures::*;
use crate::Mode;
use std::time::{Duration, Instant};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_terminal_protocol::TerminalSummary;

#[test]
fn inventory_uses_real_resources_and_selection_survives_reordering() {
    let first = session();
    let second = session();
    let mut state = crate::client_capability_support::state();
    assert_eq!(state.mode, Mode::Inventory);
    assert_eq!(state.inventory.view, yolo_tui_inventory::View::Tree);
    crate::tabs::terminal_picker::open(&mut state);
    assert!(state.visible().is_empty());
    state.move_selection(true);
    assert_eq!(state.selected, None);
    state.replace_terminals(vec![first.clone(), second.clone()]);
    let initial = state.selected.unwrap();
    state.move_selection(true);
    assert_ne!(state.selected.unwrap(), initial);
    state.move_selection(true);
    state.move_selection(false);
    assert_eq!(state.selected, Some(initial));
    state.replace_terminals(vec![second, first]);
    assert_eq!(state.selected, Some(initial));
    for filter in [
        SESSION_NAME.to_lowercase(),
        SESSION_DIRECTORY.into(),
        initial.to_string(),
    ] {
        state.filter = filter;
        state.reconcile_selection();
        assert!(state.visible().contains(&initial));
    }
    state.filter = "absent-value".into();
    state.reconcile_selection();
    assert!(state.visible().is_empty());
    assert_eq!(state.selected, None);
}

#[test]
fn leases_expire_disconnect_and_cannot_follow_replaced_runtime() {
    let current = session();
    let address = current.fields().address;
    let grant = grant(&current);
    let now = Instant::now();
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![current.clone()]);
    state.record_grant(grant, now);
    assert_eq!(state.grant(address, now), None);
    state.control_generation = Some(uuid::Uuid::new_v4());
    assert_eq!(state.grant(address, now), Some(grant));
    assert_eq!(
        state.grant(address, now + Duration::from_millis(TTL_MS)),
        None
    );
    state.bind_focused(Some(address)).unwrap();
    assert_eq!(state.focused_terminal(), Some(address));
    let mut fields = current.fields().clone();
    fields.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new();
    state.replace_terminals(vec![TerminalSummary::new(fields).unwrap()]);
    assert!(state.writers.is_empty());
    state.record_grant(grant, now);
    state.disconnected();
    assert!(state.control_generation.is_none());
    assert!(state.writers.is_empty());
    assert_eq!(state.grant(address, now), None);
}
