use super::fixtures;
use crate::{State, network::Job};
use ratatui::layout::Rect;
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{LeaseGrant, ResizeRequest};
use yolo_tui_render::PaneViewport;

const INITIAL_WIDTH: u16 = 80;
const RETURNED_WIDTH: u16 = 80;
const DRAGGED_WIDTH: u16 = 100;
const OUTER_HEIGHT: u16 = 12;
const INNER_HEIGHT: u16 = 10;

fn view(address: yolo_hub_identity::TerminalAddress, width: u16) -> PaneViewport {
    PaneViewport {
        pane: Uuid::new_v4(),
        terminal: Some(address),
        outer: Rect::new(0, 0, width, OUTER_HEIGHT),
        inner: Rect::new(0, 0, width, INNER_HEIGHT),
    }
}

fn ready_state() -> (State, yolo_hub_identity::TerminalAddress) {
    let session = fixtures::session();
    let address = session.fields().address;
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.replace_terminals(vec![session.clone()]);
    state.record_grant(fixtures::grant(&session), Instant::now());
    (state, address)
}

#[test]
fn returning_to_in_flight_geometry_does_not_enqueue_a_second_resize() {
    let (mut state, address) = ready_state();
    let (jobs, mut received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);

    crate::resize::apply(&mut state, &jobs, view(address, INITIAL_WIDTH));
    let _ = received.try_recv();
    crate::resize::apply(&mut state, &jobs, view(address, DRAGGED_WIDTH));
    crate::resize::apply(&mut state, &jobs, view(address, RETURNED_WIDTH));
    assert_eq!(
        state.last_size(address).expect("last size").cols(),
        RETURNED_WIDTH
    );
    assert!(received.try_recv().is_err());

    let request = state.resize_in_flight(address).expect("resize in flight");
    crate::resize::completed(
        &mut state,
        &yolo_hub_terminal_protocol::ResizeRequest {
            request_id: request.request_id,
            grant: request.grant,
            size: request.size,
        },
    );
    assert!(!state.is_resize_pending(address));
    assert!(state.is_resync(address));
}

#[test]
fn queue_full_resize_is_retained_until_the_next_draw_can_admit_it() {
    let (mut state, address) = ready_state();
    let (jobs, mut received) = mpsc::channel(1);
    jobs.try_send(Job::Refresh {
        request_id: Uuid::new_v4(),
    })
    .unwrap();

    crate::resize::apply(&mut state, &jobs, view(address, INITIAL_WIDTH));
    assert!(state.is_resize_pending(address));
    assert!(!state.resize_in_flight(address).is_some());
    let _ = received.try_recv();

    crate::resize::apply(&mut state, &jobs, view(address, INITIAL_WIDTH));
    assert!(state.resize_in_flight(address).is_some());
    assert!(matches!(
        received.try_recv().unwrap(),
        Job::Call {
            command: Command::Resize { .. },
            ..
        }
    ));
}

#[test]
fn lease_renewal_keeps_the_resize_request_fence() {
    let session = fixtures::session();
    let address = session.fields().address;
    let grant = fixtures::grant(&session);
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.replace_terminals(vec![session]);
    state.record_grant(grant, Instant::now());
    let (jobs, mut received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);

    crate::resize::apply(&mut state, &jobs, view(address, INITIAL_WIDTH));
    let request = match received.try_recv().unwrap() {
        Job::Call {
            command: Command::Resize { request },
            ..
        } => request,
        _ => panic!("resize job expected"),
    };
    let mut fields = *grant.fields();
    fields.expires_at_ms += 1;
    let renewed = LeaseGrant::new(fields).unwrap();
    state.record_grant(renewed, Instant::now());
    assert_eq!(
        state
            .resize_in_flight(address)
            .expect("resize in flight")
            .request_id,
        request.request_id
    );

    crate::resize::completed(
        &mut state,
        &ResizeRequest {
            request_id: request.request_id,
            grant: request.grant,
            size: request.size,
        },
    );
    assert!(!state.is_resize_pending(address));
    assert!(state.is_resync(address));
}
