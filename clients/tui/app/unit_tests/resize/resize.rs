use super::fixtures;
use crate::{
    events::apply,
    network::{ObservationUpdate, Update},
    resize::PendingResize,
};
use ratatui::layout::Rect;
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    ResizeRequest, TerminalSize, TerminalSnapshot, TerminalSnapshotFields,
};
use yolo_tui_terminal::TerminalSurface;

fn viewport(
    address: yolo_hub_identity::TerminalAddress,
    width: u16,
) -> yolo_tui_render::PaneViewport {
    yolo_tui_render::PaneViewport {
        pane: uuid::Uuid::new_v4(),
        terminal: Some(address),
        outer: Rect::new(0, 0, width, 12),
        inner: Rect::new(0, 0, width, 10),
    }
}

#[test]
fn control_is_not_ready_until_snapshot_dimensions_are_confirmed() {
    let session = fixtures::session();
    let address = session.fields().address;
    let target = session.fields().runtime.unwrap().target;
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![session.clone()]);
    state.record_grant(fixtures::grant(&session), Instant::now());
    assert!(!state.input_ready(address));
    let snapshot = TerminalSnapshot::new(TerminalSnapshotFields {
        target,
        barrier_sequence: 0,
        size: TerminalSize::default(),
        data: vec![],
    })
    .unwrap();
    state
        .surfaces
        .insert(address, TerminalSurface::new(&snapshot));
    assert!(!state.input_ready(address));
    state.set_last_size(address, TerminalSize::default());
    assert!(state.input_ready(address));
    state.mark_resize_pending(address);
    assert!(!state.input_ready(address));
    state.clear_all_resize_pending();
    state.mark_resync(address);
    assert!(!state.input_ready(address));
}

#[test]
fn resize_confirmation_requests_a_snapshot_and_renewal_does_not_repeat_resize() {
    let session = fixtures::session();
    let address = session.fields().address;
    let grant = fixtures::grant(&session);
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.replace_terminals(vec![session]);
    state.record_grant(grant, Instant::now());
    state.set_last_size(address, TerminalSize::default());
    state.mark_resize_pending(address);
    let (jobs, _) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let request = ResizeRequest {
        request_id: uuid::Uuid::new_v4(),
        grant,
        size: TerminalSize::default(),
    };
    state.set_resize_in_flight(
        address,
        PendingResize {
            request_id: request.request_id,
            grant: request.grant,
            size: request.size,
        },
    );
    apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Resize { request },
            reply: Reply::Completed {},
        },
    );
    assert!(!state.is_resize_pending(address));
    assert!(state.is_resync(address));
    state.record_grant(grant, Instant::now());
    assert_eq!(state.last_size(address), Some(TerminalSize::default()));
}

#[test]
fn closed_callback_from_a_replaced_subscription_cannot_clear_the_current_writer() {
    let session = fixtures::session();
    let target = session.fields().runtime.unwrap().target;
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.record_grant(fixtures::grant(&session), Instant::now());
    state.replace_terminals(vec![session]);
    let current = uuid::Uuid::new_v4();
    state.set_observer_generation(target.address, current);
    let (jobs, _) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    apply(
        &mut state,
        &jobs,
        Update::Observation {
            target,
            generation: uuid::Uuid::new_v4(),
            update: ObservationUpdate::Closed {
                message: Some(crate::constants::DISCONNECTED_LABEL.into()),
            },
        },
    );
    assert!(state.writers.contains_key(&target.address));
    apply(
        &mut state,
        &jobs,
        Update::Observation {
            target,
            generation: current,
            update: ObservationUpdate::Closed {
                message: Some(crate::constants::DISCONNECTED_LABEL.into()),
            },
        },
    );
    assert!(state.writers.contains_key(&target.address));
    assert!(state.is_offline(target.address));
}

#[test]
fn resize_drag_keeps_one_request_in_flight_and_admits_latest_geometry() {
    let session = fixtures::session();
    let address = session.fields().address;
    let grant = fixtures::grant(&session);
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.replace_terminals(vec![session]);
    state.record_grant(grant, Instant::now());
    let (jobs, mut received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);

    crate::resize::apply(&mut state, &jobs, viewport(address, 80));
    let first = match received.try_recv().unwrap() {
        crate::network::Job::Call {
            command: Command::Resize { request },
            ..
        } => request,
        _ => panic!("resize job expected"),
    };
    crate::resize::apply(&mut state, &jobs, viewport(address, 100));
    assert!(received.try_recv().is_err());
    assert_eq!(state.last_size(address).unwrap().cols(), 100);
    apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Resize { request: first },
            reply: Reply::Completed {},
        },
    );
    assert!(state.is_resize_pending(address));
    assert!(!state.has_resync());

    crate::resize::apply(&mut state, &jobs, viewport(address, 100));
    let second = match received.try_recv().unwrap() {
        crate::network::Job::Call {
            command: Command::Resize { request },
            ..
        } => request,
        _ => panic!("latest resize job expected"),
    };
    assert_ne!(first.request_id, second.request_id);
    apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Resize { request: first },
            reply: Reply::Completed {},
        },
    );
    assert!(state.is_resize_pending(address));
    apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Resize { request: second },
            reply: Reply::Completed {},
        },
    );
    assert!(!state.is_resize_pending(address));
    assert!(state.is_resync(address));
}
