use super::fixtures;
use crate::{Mode, State};
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_terminal_protocol::{TerminalSize, TerminalSnapshot, TerminalSnapshotFields};

fn prepared() -> (
    State,
    yolo_hub_identity::TerminalAddress,
    yolo_hub_identity::RuntimeTarget,
) {
    let session = fixtures::session();
    let address = session.fields().address;
    let target = session.fields().runtime.unwrap().target;
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.replace_terminals(vec![session.clone()]);
    state.bind_focused(Some(address)).unwrap();
    state.mode = Mode::Terminal;
    let size = TerminalSize::new(80, 24).unwrap();
    state.surfaces.insert(
        address,
        yolo_tui_terminal::TerminalSurface::new(
            &TerminalSnapshot::new(TerminalSnapshotFields {
                target,
                barrier_sequence: 0,
                size,
                data: b"screen".to_vec(),
            })
            .unwrap(),
        ),
    );
    state.set_last_size(address, size);
    state.mark_resize_pending(address);
    state.set_resize_in_flight(
        address,
        crate::resize::PendingResize {
            request_id: Uuid::new_v4(),
            grant: fixtures::grant(&session),
            size,
        },
    );
    state.mark_geometry_syncing(address);
    state.mark_resync(address);
    state.mark_offline(address);
    state.set_observer_generation(address, Uuid::new_v4());
    state.mark_renewal_pending(address);
    state.set_codex_open_attempt(address, Instant::now());
    state.inputs.snapshot_pending.insert(address);
    state.inputs.overflowed.insert(address);
    assert!(crate::terminal_input::enqueue_pregrant(
        &mut state,
        address,
        b"queued".to_vec()
    ));
    state.record_grant(fixtures::grant(&session), Instant::now());
    (state, address, target)
}

fn assert_no_runtime_state(state: &State, address: yolo_hub_identity::TerminalAddress) {
    assert!(!state.terminal_runtime.contains_key(&address));
    assert!(!state.writers.contains_key(&address));
    assert!(!state.surfaces.contains_key(&address));
    assert!(!state.last_size(address).is_some());
    assert!(!state.is_resize_pending(address));
    assert!(!state.resize_in_flight(address).is_some());
    assert!(!state.is_geometry_syncing(address));
    assert!(!state.is_resync(address));
    assert!(!state.observer_generation(address).is_some());
    assert!(!state.renewal_pending(address));
    assert!(!state.codex_open_attempt(address).is_some());
    assert!(!state.inputs.snapshot_pending.contains(&address));
    assert!(!state.inputs.overflowed.contains(&address));
    assert!(
        !state
            .inputs
            .pregrant
            .iter()
            .any(|item| { item.target == crate::terminal_input::PregrantTarget::Address(address) })
    );
    assert!(!state.is_offline(address));
    assert!(!state.ended_epoch(address).is_some());
}

#[test]
fn end_then_presentation_retirement_leaves_only_catalogue_history() {
    let (mut state, address, target) = prepared();
    state.end_runtime(target);
    assert_eq!(state.ended_epoch(address), Some(target.runtime_epoch));
    crate::tab_liveness::retire_dead(&mut state);

    assert!(state.terminals.contains_key(&address));
    assert_eq!(state.ended_epoch(address), Some(target.runtime_epoch));
    assert!(!state.writers.contains_key(&address));
    assert!(!state.surfaces.contains_key(&address));
    assert!(!state.last_size(address).is_some());
    assert!(!state.is_resize_pending(address));
    assert!(!state.resize_in_flight(address).is_some());
    assert!(!state.is_geometry_syncing(address));
    assert!(!state.is_resync(address));
    assert!(!state.observer_generation(address).is_some());
    assert!(!state.renewal_pending(address));
    assert!(!state.codex_open_attempt(address).is_some());
    assert!(!state.inputs.snapshot_pending.contains(&address));
    assert!(!state.inputs.overflowed.contains(&address));
    assert!(
        !state
            .inputs
            .pregrant
            .iter()
            .any(|item| { item.target == crate::terminal_input::PregrantTarget::Address(address) })
    );
}

#[test]
fn runtime_replacement_clears_every_incarnation_fact() {
    let (mut state, address, _) = prepared();
    state.retire_runtime(address);
    assert_no_runtime_state(&state, address);
}

#[test]
fn disconnect_clears_connection_state_but_keeps_retained_screens() {
    let (mut state, address, _) = prepared();
    state.disconnect_runtime();
    assert!(!state.writers.contains_key(&address));
    assert!(state.surfaces.contains_key(&address));
    assert!(!state.last_size(address).is_some());
    assert!(!state.is_resize_pending(address));
    assert!(!state.resize_in_flight(address).is_some());
    assert!(!state.is_geometry_syncing(address));
    assert!(!state.codex_open_attempt(address).is_some());
    assert!(!state.inputs.snapshot_pending.contains(&address));
}
