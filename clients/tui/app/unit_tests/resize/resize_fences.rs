use super::fixtures;
use crate::{events, network::Update};
use ratatui::layout::Rect;
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_rpc_protocol::Command;
use yolo_tui_render::PaneViewport;

fn view(address: yolo_hub_identity::TerminalAddress, width: u16) -> PaneViewport {
    PaneViewport {
        pane: Uuid::new_v4(),
        terminal: Some(address),
        outer: Rect::new(0, 0, width, 12),
        inner: Rect::new(0, 0, width, 10),
    }
}

#[test]
fn failed_old_resize_keeps_latest_geometry_for_retry() {
    let session = fixtures::session();
    let address = session.fields().address;
    let grant = fixtures::grant(&session);
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.replace_terminals(vec![session]);
    state.record_grant(grant, Instant::now());
    let (jobs, mut received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);

    crate::resize::apply(&mut state, &jobs, view(address, 80));
    let first = match received.try_recv().unwrap() {
        crate::network::Job::Call {
            command: Command::Resize { request },
            ..
        } => request,
        _ => panic!("resize job expected"),
    };
    crate::resize::apply(&mut state, &jobs, view(address, 100));
    events::apply(
        &mut state,
        &jobs,
        Update::Failed {
            command: Some(Command::Resize { request: first }),
            message: "resize failed".into(),
            connected: true,
            transient: false,
        },
    );
    assert!(state.is_resize_pending(address));
    assert_eq!(state.last_size(address).expect("last size").cols(), 100);

    crate::resize::apply(&mut state, &jobs, view(address, 100));
    assert!(matches!(
        received.try_recv().unwrap(),
        crate::network::Job::Call {
            command: Command::Resize { .. },
            ..
        }
    ));
}

#[test]
fn stale_failure_cannot_clear_the_newer_resize_fence() {
    let session = fixtures::session();
    let address = session.fields().address;
    let grant = fixtures::grant(&session);
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.replace_terminals(vec![session]);
    state.record_grant(grant, Instant::now());
    let (jobs, mut received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);

    crate::resize::apply(&mut state, &jobs, view(address, 80));
    let first = match received.try_recv().unwrap() {
        crate::network::Job::Call {
            command: Command::Resize { request },
            ..
        } => request,
        _ => panic!("resize job expected"),
    };
    events::apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Resize { request: first },
            reply: yolo_hub_rpc_protocol::Reply::Completed {},
        },
    );
    state.clear_all_resync();
    crate::resize::apply(&mut state, &jobs, view(address, 100));
    let second = match received.try_recv().unwrap() {
        crate::network::Job::Call {
            command: Command::Resize { request },
            ..
        } => request,
        _ => panic!("latest resize job expected"),
    };

    events::apply(
        &mut state,
        &jobs,
        Update::Failed {
            command: Some(Command::Resize { request: first }),
            message: "stale failure".into(),
            connected: true,
            transient: false,
        },
    );
    assert_eq!(
        state
            .resize_in_flight(address)
            .expect("resize in flight")
            .request_id,
        second.request_id
    );
    assert!(state.is_resize_pending(address));
    assert!(!state.has_resync());
}

#[test]
fn completion_with_the_right_identity_but_wrong_size_is_ignored() {
    let session = fixtures::session();
    let address = session.fields().address;
    let grant = fixtures::grant(&session);
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.replace_terminals(vec![session]);
    state.record_grant(grant, Instant::now());
    let (jobs, mut received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);

    crate::resize::apply(&mut state, &jobs, view(address, 80));
    let request = match received.try_recv().unwrap() {
        crate::network::Job::Call {
            command: Command::Resize { request },
            ..
        } => request,
        _ => panic!("resize job expected"),
    };
    let mut forged = request;
    forged.size = yolo_hub_terminal_protocol::TerminalSize::new(100, 10).unwrap();
    events::apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Resize { request: forged },
            reply: yolo_hub_rpc_protocol::Reply::Completed {},
        },
    );

    assert!(state.resize_in_flight(address).is_some());
    assert!(state.is_resize_pending(address));
    assert!(!state.has_resync());
}

#[test]
fn catalog_removal_clears_resize_state_for_the_removed_writer() {
    let session = fixtures::session();
    let address = session.fields().address;
    let grant = fixtures::grant(&session);
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.replace_terminals(vec![session]);
    state.record_grant(grant, Instant::now());
    let (jobs, mut received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    crate::resize::apply(&mut state, &jobs, view(address, 80));
    let _ = received.try_recv();
    assert!(state.resize_in_flight(address).is_some());

    state.replace_terminals(Vec::new());

    assert!(!state.writers.contains_key(&address));
    assert!(!state.resize_in_flight(address).is_some());
    assert!(!state.is_resize_pending(address));
    assert!(!state.last_size(address).is_some());
}
